#!/usr/bin/env python3
"""协议转换的端到端验证。

单元测试只能证明"函数输出长这样"，证明不了"官方 API 会不会收"。
这里起一个**严格校验**的假上游：它会像官方 Anthropic / Gemini 那样挑剔，
收到不合规的 body 就 400 并说明哪里不对。请求从网关走一圈，
如果我们的转换漏了东西，这里一定红。

校验点（都是官方 API 真实会拒绝的）：
  Anthropic
    - messages[].role 只能是 user / assistant（不能有 tool）
    - 必须有 max_tokens
    - content 必须是 block 数组
    - tool 结果必须是 {"type":"tool_result","tool_use_id":...}
    - 不能带 stream_options / n
  Gemini
    - 必须有 contents（不能是 messages）
    - contents[].role 只能是 user / model
    - functionResponse 必须是函数名，不是 call id

用法：
    python scripts/test_convert.py --all
"""
import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

MOCK_PORT = 18101
_OPENER = urllib.request.build_opener(urllib.request.ProxyHandler({}))


class Strict(BaseHTTPRequestHandler):
    """像官方 API 一样挑剔的假上游"""
    protocol_version = "HTTP/1.1"
    errors = []          # 校验失败记录，测试结束时报告

    def log_message(self, *a):
        pass

    def _json(self, code, obj):
        body = json.dumps(obj, ensure_ascii=False).encode()
        self.send_response(code)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _reject(self, msg):
        Strict.errors.append(msg)
        # 模仿官方错误结构
        self._json(400, {"type": "error", "error": {"type": "invalid_request_error", "message": msg}})

    def do_POST(self):
        n = int(self.headers.get("content-length") or 0)
        raw = self.rfile.read(n)
        try:
            req = json.loads(raw or b"{}")
        except Exception:
            return self._reject("body 不是合法 JSON")

        if "/messages" in self.path:
            return self.anthropic(req)
        if "generateContent" in self.path:
            return self.gemini(req)
        if "chat/completions" in self.path:
            return self.openai_passthrough(req)
        return self._reject(f"未知路径 {self.path}")

    # ---------- OpenAI（直传，不做转换） ----------
    def openai_passthrough(self, req):
        msgs = req.get("messages") or []
        if not msgs:
            return self._reject("缺少 messages")
        if "stream_options" in req:
            return self._reject("不该出现 stream_options")
        # 直传模式下 tool 角色应该原样保留（不该被转成别的）
        has_tool = any(m.get("role") == "tool" for m in msgs)
        if has_tool:
            return self._json(200, {
                "id": "chatcmpl-strict", "object": "chat.completion", "created": 0,
                "model": req.get("model", "m"),
                "choices": [{"index": 0, "finish_reason": "tool_calls", "message": {
                    "role": "assistant", "content": None,
                    "tool_calls": [{"id": "call_back", "type": "function",
                                    "function": {"name": "get_weather",
                                                 "arguments": "{\"city\":\"上海\"}"}}]}}],
                "usage": {"prompt_tokens": 11, "completion_tokens": 5, "total_tokens": 16},
            })
        self._json(200, {
            "id": "chatcmpl-strict", "object": "chat.completion", "created": 0,
            "model": req.get("model", "m"),
            "choices": [{"index": 0, "finish_reason": "stop",
                         "message": {"role": "assistant", "content": "转换成功"}}],
            "usage": {"prompt_tokens": 11, "completion_tokens": 5, "total_tokens": 16},
        })

    # ---------- Anthropic ----------
    def anthropic(self, req):
        if "messages" not in req:
            return self._reject("缺少 messages")
        if "max_tokens" not in req:
            return self._reject("缺少 max_tokens（Anthropic 必填）")
        for k in ("stream_options", "n"):
            if k in req:
                return self._reject(f"不该出现 {k}")
        msgs = req["messages"]
        if not isinstance(msgs, list) or not msgs:
            return self._reject("messages 必须是数组")
        for i, m in enumerate(msgs):
            role = m.get("role")
            if role not in ("user", "assistant"):
                return self._reject(f"messages[{i}].role = {role!r}，只允许 user / assistant")
            if not isinstance(m.get("content"), list):
                return self._reject(f"messages[{i}].content 必须是 block 数组")
            for b in m["content"]:
                t = b.get("type")
                if t == "tool_result" and not b.get("tool_use_id"):
                    return self._reject(f"messages[{i}] 的 tool_result 缺 tool_use_id")
                if t == "tool_use" and not isinstance(b.get("input"), dict):
                    return self._reject(f"messages[{i}] 的 tool_use.input 必须是对象")
        if "system" in req and not isinstance(req["system"], str):
            return self._reject("system 必须是字符串")

        # 合规：如果有 tool_use，就回一个 tool_use 响应
        has_tool = any(
            b.get("type") == "tool_use"
            for m in msgs for b in (m.get("content") or [])
        )
        if has_tool:
            return self._json(200, {
                "id": "msg_strict", "type": "message", "role": "assistant",
                "model": req.get("model", "m"),
                "content": [{"type": "tool_use", "id": "call_back",
                             "name": "get_weather", "input": {"city": "上海"}}],
                "stop_reason": "tool_use",
                "usage": {"input_tokens": 11, "output_tokens": 5},
            })
        self._json(200, {
            "id": "msg_strict", "type": "message", "role": "assistant",
            "model": req.get("model", "m"),
            "content": [{"type": "text", "text": "转换成功"}],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 11, "output_tokens": 5},
        })

    # ---------- Gemini ----------
    def gemini(self, req):
        if "contents" not in req:
            return self._reject("缺少 contents（Gemini 用 contents，不是 messages）")
        if "messages" in req:
            return self._reject("不该出现 messages")
        cs = req["contents"]
        if not isinstance(cs, list) or not cs:
            return self._reject("contents 必须是数组")
        for i, c in enumerate(cs):
            role = c.get("role")
            if role not in ("user", "model"):
                return self._reject(f"contents[{i}].role = {role!r}，只允许 user / model")
            parts = c.get("parts")
            if not isinstance(parts, list) or not parts:
                return self._reject(f"contents[{i}].parts 必须是非空数组")
            for p in parts:
                fr = p.get("functionResponse")
                if fr and not fr.get("name"):
                    return self._reject(f"contents[{i}] 的 functionResponse 缺 name")
                fc = p.get("functionCall")
                if fc and not isinstance(fc.get("args"), dict):
                    return self._reject(f"contents[{i}] 的 functionCall.args 必须是对象")

        has_call = any(
            "functionCall" in p
            for c in cs for p in (c.get("parts") or [])
        )
        if has_call:
            return self._json(200, {"candidates": [{
                "content": {"role": "model", "parts": [
                    {"functionCall": {"name": "get_weather", "args": {"city": "上海"}}}]},
                "finishReason": "STOP"}],
                "usageMetadata": {"promptTokenCount": 11, "candidatesTokenCount": 5}})
        self._json(200, {"candidates": [{
            "content": {"role": "model", "parts": [{"text": "转换成功"}]},
            "finishReason": "STOP"}],
            "usageMetadata": {"promptTokenCount": 11, "candidatesTokenCount": 5}})


TOOL_REQ = {
    "model": "any-model",
    "messages": [
        {"role": "system", "content": "你是助手"},
        {"role": "user", "content": "北京天气怎么样"},
        {"role": "assistant", "content": None, "tool_calls": [
            {"id": "call_1", "type": "function",
             "function": {"name": "get_weather", "arguments": "{\"city\":\"北京\"}"}}
        ]},
        {"role": "tool", "tool_call_id": "call_1", "content": "晴，25 度"},
    ],
    "tools": [{"type": "function", "function": {
        "name": "get_weather", "description": "查天气",
        "parameters": {"type": "object", "properties": {"city": {"type": "string"}}}}}],
    "tool_choice": "auto",
}


def post(port, payload):
    req = urllib.request.Request(
        f"http://127.0.0.1:{port}/v1/chat/completions",
        data=json.dumps(payload).encode(),
        headers={"content-type": "application/json"},
    )
    try:
        with _OPENER.open(req, timeout=30) as r:
            return json.loads(r.read().decode("utf-8", "replace"))
    except urllib.error.HTTPError as e:
        return {"_http_error": e.code, "_body": e.read().decode("utf-8", "replace")}


def run(protocol, exe, port):
    print(f"\n{'='*62}\n  协议转换：{protocol}\n{'='*62}")
    Strict.errors = []
    tmp = tempfile.mkdtemp(prefix=f"conv-{protocol}-")
    with open(os.path.join(tmp, "upstreams.json"), "w", encoding="utf-8") as f:
        json.dump([{"id": "strict", "name": "Strict",
                    "baseUrl": f"http://127.0.0.1:{MOCK_PORT}",
                    "apiKey": "k", "protocol": protocol, "enabled": True}], f)

    env = dict(os.environ, WB_AXIAO_DATA=tmp)
    p = subprocess.Popen([exe, "--serve-only", "--port", str(port)],
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env)
    for _ in range(60):
        time.sleep(0.25)
        try:
            _OPENER.open(f"http://127.0.0.1:{port}/health", timeout=1).read()
            break
        except Exception:
            pass
    else:
        print("  [FAIL] 网关没起来")
        p.kill(); shutil.rmtree(tmp, ignore_errors=True)
        return False

    ok = True

    def check(name, cond, detail=""):
        nonlocal ok
        print(f"  [{'PASS' if cond else 'FAIL'}] {name} {detail}")
        if not cond:
            ok = False

    # ---- 1. 纯文本 ----
    plain = {"model": "any-model", "messages": [
        {"role": "system", "content": "你是助手"},
        {"role": "user", "content": "你好"}]}
    r = post(port, plain)
    check("纯文本请求被上游接受", "_http_error" not in r,
          r.get("_body", "")[:120] if "_http_error" in r else "")
    if "_http_error" not in r:
        txt = (r.get("choices") or [{}])[0].get("message", {}).get("content", "")
        check("响应被转回 OpenAI 格式", txt == "转换成功", f"content={txt!r}")

    # ---- 2. 带工具调用 ----
    r2 = post(port, TOOL_REQ)
    check("工具调用请求被上游接受（没被 400）", "_http_error" not in r2,
          r2.get("_body", "")[:200] if "_http_error" in r2 else "")
    if "_http_error" not in r2:
        choice = (r2.get("choices") or [{}])[0]
        tcs = choice.get("message", {}).get("tool_calls") or []
        check("上游的 tool 调用被转回 OpenAI tool_calls", len(tcs) > 0,
              f"finish_reason={choice.get('finish_reason')}")
        if tcs:
            check("函数名对上", tcs[0]["function"]["name"] == "get_weather",
                  tcs[0]["function"]["name"])
            check("finish_reason 变成 tool_calls",
                  choice.get("finish_reason") == "tool_calls", choice.get("finish_reason"))

    if Strict.errors:
        print("\n  上游的校验意见：")
        for e in Strict.errors:
            print(f"    - {e}")

    p.kill()
    shutil.rmtree(tmp, ignore_errors=True)
    return ok


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--protocol", default="openai", choices=["openai", "anthropic", "gemini"])
    ap.add_argument("--all", action="store_true")
    ap.add_argument("--port", type=int, default=7931)
    ap.add_argument("--exe", default=r"D:\axiao-build\release\axiao-gateway.exe")
    args = ap.parse_args()

    if not os.path.exists(args.exe):
        print(f"[FAIL] 找不到 {args.exe}")
        return 1

    srv = ThreadingHTTPServer(("127.0.0.1", MOCK_PORT), Strict)
    srv.daemon_threads = True
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    print(f"[mock] 严格校验上游 :{MOCK_PORT}")

    targets = ["openai", "anthropic", "gemini"] if args.all else [args.protocol]
    results = []
    for i, pr in enumerate(targets):
        results.append((pr, run(pr, args.exe, args.port + i)))

    print(f"\n{'='*62}")
    for pr, r in results:
        print(f"  {pr:<10} {'通过' if r else '失败'}")
    allok = all(r for _, r in results)
    print("全部通过" if allok else "有失败项")
    return 0 if allok else 1


if __name__ == "__main__":
    sys.exit(main())
