#!/usr/bin/env python3
"""错误分类 + 上下文预检的端到端验证。

这三件事看似相近，处理方式却完全相反，所以必须分开验证：

1. 上下文超长（400）→ **应该换渠道**，因为换个窗口更大的模型一定能成
2. 输入明显超过窗口 → **应该直接跳过**，连请求都不发（省一次往返和输入费）
3. 参数错误（400）→ **不该换渠道**，换谁都一样，白烧配额

用法：
    python scripts/test_errors.py
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

CTX_PORT = 18141    # 返回"上下文超长"
PARAM_PORT = 18142  # 返回"参数不支持"
GOOD_PORT = 18143   # 正常
_OPENER = urllib.request.build_opener(urllib.request.ProxyHandler({}))


def make_handler(kind):
    class H(BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"
        calls = 0

        def log_message(self, *a):
            pass

        def _send(self, code, obj):
            body = json.dumps(obj, ensure_ascii=False).encode()
            self.send_response(code)
            self.send_header("content-type", "application/json")
            self.send_header("content-length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_POST(self):
            n = int(self.headers.get("content-length") or 0)
            raw = self.rfile.read(n)
            try:
                req = json.loads(raw or b"{}")
            except Exception:
                req = {}
            H.calls += 1
            if kind == "ctx":
                # 措辞照抄 OpenAI 真实报错
                return self._send(400, {"error": {
                    "message": "This model's maximum context length is 8192 tokens. However, "
                               "your messages resulted in 12000 tokens.",
                    "type": "invalid_request_error", "code": "context_length_exceeded"}})
            if kind == "param":
                return self._send(400, {"error": {
                    "message": "Unsupported value: 'temperature' does not support 3.0",
                    "type": "invalid_request_error", "param": "temperature"}})
            self._send(200, {
                "id": "chatcmpl-ok", "object": "chat.completion", "created": 0,
                "model": req.get("model", "m"),
                "choices": [{"index": 0, "finish_reason": "stop",
                             "message": {"role": "assistant", "content": "好渠道答的"}}],
                "usage": {"prompt_tokens": 5, "completion_tokens": 3, "total_tokens": 8}})

    return H


def start_gateway(exe, port, upstreams):
    tmp = tempfile.mkdtemp(prefix="err-")
    with open(os.path.join(tmp, "upstreams.json"), "w", encoding="utf-8") as f:
        json.dump(upstreams, f)
    env = dict(os.environ, WB_AXIAO_DATA=tmp)
    p = subprocess.Popen([exe, "--serve-only", "--port", str(port)],
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env)
    for _ in range(60):
        time.sleep(0.25)
        try:
            _OPENER.open(f"http://127.0.0.1:{port}/health", timeout=1).read()
            return p, tmp
        except Exception:
            pass
    p.kill()
    shutil.rmtree(tmp, ignore_errors=True)
    return None, tmp


def post(port, payload):
    req = urllib.request.Request(
        f"http://127.0.0.1:{port}/v1/chat/completions",
        data=json.dumps(payload).encode(),
        headers={"content-type": "application/json"})
    try:
        with _OPENER.open(req, timeout=40) as r:
            return json.loads(r.read().decode()), 200
    except urllib.error.HTTPError as e:
        return {"_body": e.read().decode()}, e.code


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=7981)
    ap.add_argument("--exe", default=r"D:\axiao-build\release\axiao-gateway.exe")
    args = ap.parse_args()
    if not os.path.exists(args.exe):
        print(f"[FAIL] 找不到 {args.exe}")
        return 1

    handlers = {}
    for kind, port in (("ctx", CTX_PORT), ("param", PARAM_PORT), ("good", GOOD_PORT)):
        H = make_handler(kind)
        handlers[kind] = H
        srv = ThreadingHTTPServer(("127.0.0.1", port), H)
        srv.daemon_threads = True
        threading.Thread(target=srv.serve_forever, daemon=True).start()

    ok = True

    def check(name, cond, detail=""):
        nonlocal ok
        print(f"  [{'PASS' if cond else 'FAIL'}] {name} {detail}")
        if not cond:
            ok = False

    # ---------- 1. 上下文超长 → 应该换渠道 ----------
    print("\n1) 上游说「上下文超长」，应该换一个渠道（不是直接失败）")
    handlers["ctx"].calls = 0
    handlers["good"].calls = 0
    ups = [
        {"id": "ctx", "name": "窄窗口", "baseUrl": f"http://127.0.0.1:{CTX_PORT}/v1",
         "apiKey": "k", "protocol": "openai", "enabled": True},
        {"id": "good", "name": "大窗口", "baseUrl": f"http://127.0.0.1:{GOOD_PORT}/v1",
         "apiKey": "k", "protocol": "openai", "enabled": True},
    ]
    gw, tmp = start_gateway(args.exe, args.port, ups)
    if not gw:
        print("[FAIL] 网关没起来")
        return 1
    # 用 gpt-4o（128k 窗口），请求很短，预检不会拦；窄窗口那个是上游自己拒绝的
    okc = 0
    for i in range(4):
        d, c = post(args.port, {"model": "gpt-4o",
                                "messages": [{"role": "user", "content": f"超长测试{i}"}]})
        if c == 200 and d.get("choices"):
            okc += 1
    check("全部成功（说明上下文超长触发了降级）", okc == 4, f"{okc}/4")
    check("好渠道被调用过", handlers["good"].calls > 0, f"calls={handlers['good'].calls}")

    # 关键：上下文超长不该让渠道进冷却（不是渠道的错）
    try:
        h = json.loads(_OPENER.open(f"http://127.0.0.1:{args.port}/health", timeout=3).read())
        bk = {x["id"]: x for x in h.get("breaker", [])}
        check("窄窗口渠道没有被冷却（不是它的错）",
              bk.get("ctx", {}).get("cooling") is not True, str(bk.get("ctx")))
    except Exception as e:
        check("窄窗口渠道没有被冷却", False, str(e))
    gw.kill(); shutil.rmtree(tmp, ignore_errors=True)

    # ---------- 2. 预检：装不下就别发 ----------
    print("\n2) 输入明显超过模型窗口，应该直接跳过、一个请求都不发")
    handlers["good"].calls = 0
    ups = [
        {"id": "good", "name": "大窗口", "baseUrl": f"http://127.0.0.1:{GOOD_PORT}/v1",
         "apiKey": "k", "protocol": "openai", "enabled": True},
    ]
    gw, tmp = start_gateway(args.exe, args.port + 1, ups)
    if gw:
        big = "长" * 50000   # 约 5 万 token，gpt-4 窗口只有 8192
        d, c = post(args.port + 1, {"model": "gpt-4",
                                    "messages": [{"role": "user", "content": big}]})
        check("请求被拒绝而不是硬发出去", c >= 400, f"code={c}")
        check("上游一次都没被调用", handlers["good"].calls == 0,
              f"calls={handlers['good'].calls}")
        body = d.get("_body", "")
        check("错误里说清了是窗口问题", "窗口" in body or "context" in body.lower(),
              body[:120])

        # 同一渠道换成大窗口模型，短请求应该正常
        d2, c2 = post(args.port + 1, {"model": "gpt-4o",
                                      "messages": [{"role": "user", "content": "短请求"}]})
        check("换成 128k 窗口的模型就正常了", c2 == 200, f"code={c2}")
        gw.kill()
    shutil.rmtree(tmp, ignore_errors=True)

    # ---------- 3. 参数错误 → 不该换渠道 ----------
    print("\n3) 参数不被接受（400），不该换渠道——换谁都一样")
    handlers["param"].calls = 0
    handlers["good"].calls = 0
    ups = [
        {"id": "param", "name": "挑剔渠道", "baseUrl": f"http://127.0.0.1:{PARAM_PORT}/v1",
         "apiKey": "k", "protocol": "openai", "enabled": True},
        {"id": "good", "name": "好渠道", "baseUrl": f"http://127.0.0.1:{GOOD_PORT}/v1",
         "apiKey": "k", "protocol": "openai", "enabled": True},
    ]
    gw, tmp = start_gateway(args.exe, args.port + 2, ups)
    if gw:
        codes = []
        for i in range(6):
            _, c = post(args.port + 2, {"model": "gpt-4o",
                                        "messages": [{"role": "user", "content": f"参数测试{i}"}]})
            codes.append(c)
        check("出现过 400（没有对参数错误降级）", 400 in codes, f"码序列={codes}")
        gw.kill()
    shutil.rmtree(tmp, ignore_errors=True)

    print("\n" + ("全部通过" if ok else "有失败项"))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
