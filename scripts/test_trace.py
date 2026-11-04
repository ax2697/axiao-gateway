#!/usr/bin/env python3
"""请求追踪 ID 的端到端验证。

要求是"三处能对上"：客户端拿到的响应头、账本里的一行、日志里的若干行。
任何一处对不上，排障时就还是得靠猜时间。

验证项：
1. 响应头返回 X-Request-Id
2. 客户端自带 X-Request-Id 时原样沿用（能和对方系统日志串起来）
3. 账本该行带同一个 requestId
4. 网关日志里能用这个 id 搜到
5. 流式请求同样带头
6. 非法 id（超长 / 含控制字符）被忽略并换成新生成的

用法：
    python scripts/test_trace.py
"""
import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

MOCK_PORT = 18131
_OPENER = urllib.request.build_opener(urllib.request.ProxyHandler({}))


class Mock(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *a):
        pass

    def do_POST(self):
        n = int(self.headers.get("content-length") or 0)
        raw = self.rfile.read(n)
        try:
            req = json.loads(raw or b"{}")
        except Exception:
            req = {}
        if req.get("stream"):
            self.send_response(200)
            self.send_header("content-type", "text/event-stream")
            self.send_header("connection", "close")
            self.end_headers()
            self.close_connection = True
            self.wfile.write(
                b'data: {"id":"c","object":"chat.completion.chunk","created":0,"model":"m",'
                b'"choices":[{"index":0,"delta":{"content":"hi"},"finish_reason":"stop"}]}\n\n')
            self.wfile.write(b"data: [DONE]\n\n")
            self.wfile.flush()
            return
        body = json.dumps({
            "id": "chatcmpl-mock", "object": "chat.completion", "created": 0,
            "model": req.get("model", "m"),
            "choices": [{"index": 0, "finish_reason": "stop",
                         "message": {"role": "assistant", "content": "答案"}}],
            "usage": {"prompt_tokens": 3, "completion_tokens": 2, "total_tokens": 5},
        }).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def call(port, payload, rid=None, stream=False):
    """返回 (响应头 dict, 原始响应)"""
    headers = {"content-type": "application/json"}
    if rid:
        headers["X-Request-Id"] = rid
    req = urllib.request.Request(
        f"http://127.0.0.1:{port}/v1/chat/completions",
        data=json.dumps(payload).encode(),
        headers=headers,
    )
    try:
        with _OPENER.open(req, timeout=30) as r:
            return dict(r.headers), r.read().decode("utf-8", "replace")
    except urllib.error.HTTPError as e:
        return dict(e.headers), e.read().decode("utf-8", "replace")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=7971)
    ap.add_argument("--exe", default=r"D:\axiao-build\release\axiao-gateway.exe")
    args = ap.parse_args()

    if not os.path.exists(args.exe):
        print(f"[FAIL] 找不到 {args.exe}")
        return 1

    srv = ThreadingHTTPServer(("127.0.0.1", MOCK_PORT), Mock)
    srv.daemon_threads = True
    threading.Thread(target=srv.serve_forever, daemon=True).start()

    tmp = tempfile.mkdtemp(prefix="trace-")
    with open(os.path.join(tmp, "upstreams.json"), "w", encoding="utf-8") as f:
        json.dump([{"id": "mock", "name": "Mock",
                    "baseUrl": f"http://127.0.0.1:{MOCK_PORT}/v1",
                    "apiKey": "k", "protocol": "openai", "enabled": True}], f)

    env = dict(os.environ, WB_AXIAO_DATA=tmp)
    p = subprocess.Popen([args.exe, "--serve-only", "--port", str(args.port)],
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env)
    for _ in range(60):
        time.sleep(0.25)
        try:
            _OPENER.open(f"http://127.0.0.1:{args.port}/health", timeout=1).read()
            break
        except Exception:
            pass
    else:
        print("[FAIL] 网关没起来")
        p.kill(); shutil.rmtree(tmp, ignore_errors=True)
        return 1

    ok = True

    def check(name, cond, detail=""):
        nonlocal ok
        print(f"  [{'PASS' if cond else 'FAIL'}] {name} {detail}")
        if not cond:
            ok = False

    def hdr(headers, name):
        for k, v in headers.items():
            if k.lower() == name.lower():
                return v
        return None

    # ---- 1. 自动生成 ----
    print("\n1) 不传 id，网关应自己生成并回填")
    h1, _ = call(args.port, {"model": "m", "messages": [{"role": "user", "content": "追踪测试A"}]})
    rid1 = hdr(h1, "X-Request-Id")
    check("响应头有 X-Request-Id", bool(rid1), str(rid1))
    check("格式是 req_ + 8 位十六进制", bool(rid1 and re.fullmatch(r"req_[0-9a-f]{8}", rid1)), str(rid1))

    # ---- 2. 沿用客户端 id ----
    print("\n2) 客户端自带 id，应原样沿用")
    mine = "req_mycustom1"
    h2, _ = call(args.port, {"model": "m", "messages": [{"role": "user", "content": "追踪测试B"}]}, rid=mine)
    got = hdr(h2, "X-Request-Id")
    check("响应头回显客户端的 id", got == mine, f"{got}")

    # ---- 3. 账本里能对上 ----
    print("\n3) 账本该行带同一个 requestId")
    time.sleep(0.4)
    ledger = os.path.join(tmp, "ledger.jsonl")
    rows = []
    if os.path.exists(ledger):
        for line in open(ledger, encoding="utf-8"):
            line = line.strip()
            if not line:
                continue
            try:
                rows.append(json.loads(line))
            except Exception:
                pass
    hit = [r for r in rows if r.get("requestId") == mine]
    check("账本里有这条 id", len(hit) == 1, f"找到 {len(hit)} 条")
    if hit:
        check("账本同时记录了上游", hit[0].get("upstream") == "mock", str(hit[0].get("upstream")))
        check("账本记了 token", hit[0].get("completionTokens") == 2, str(hit[0].get("completionTokens")))

    # ---- 4. 日志里能搜到 ----
    print("\n4) 日志里能用这个 id 搜到")
    log = os.path.join(tmp, "gateway.log")
    text = open(log, encoding="utf-8").read() if os.path.exists(log) else ""
    hits = [l for l in text.splitlines() if mine in l]
    check("日志里出现该 id", len(hits) > 0, f"{len(hits)} 行")
    check("日志格式带方括号标记", any(f"[{mine}]" in l for l in hits))

    # ---- 5. 流式也带头 ----
    print("\n5) 流式请求也带 X-Request-Id")
    h3, raw = call(args.port, {"model": "m", "stream": True,
                               "messages": [{"role": "user", "content": "流式追踪"}]})
    rid3 = hdr(h3, "X-Request-Id")
    check("流式响应头有 id", bool(rid3), str(rid3))
    check("流式内容正常", "[DONE]" in raw, raw[:40])

    # ---- 6. 非法 id 被忽略 ----
    print("\n6) 非法 id（超长）应被忽略并换新的")
    h4, _ = call(args.port, {"model": "m", "messages": [{"role": "user", "content": "非法id"}]},
                 rid="x" * 200)
    rid4 = hdr(h4, "X-Request-Id")
    check("超长 id 没被采用", rid4 and len(rid4) <= 64, f"len={len(rid4 or '')}")
    check("换成了新生成的 id", bool(rid4 and rid4.startswith("req_")), str(rid4))

    p.kill()
    shutil.rmtree(tmp, ignore_errors=True)
    print("\n" + ("全部通过" if ok else "有失败项"))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
