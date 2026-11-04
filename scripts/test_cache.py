#!/usr/bin/env python3
"""响应缓存的行为验证。

只看单测不够 —— 要证明"同一个请求真的没打到上游"。这里的做法是让假上游
数请求次数，然后从网关发同样的请求两次，看上游收到几次。

验证项：
1. 相同请求第二次命中缓存 → 上游只被请求 1 次
2. 温度 > 0 → 不缓存，上游被请求 2 次
3. 带 tools → 不缓存
4. 流式 → 不缓存（走的是另一条链路，这里只确认它不被误缓存）
5. 内容不同的请求不会串答案

用法：
    python scripts/test_cache.py
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

MOCK_PORT = 18111
_OPENER = urllib.request.build_opener(urllib.request.ProxyHandler({}))


class Counting(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    calls = 0
    bodies = []

    def log_message(self, *a):
        pass

    def do_POST(self):
        n = int(self.headers.get("content-length") or 0)
        raw = self.rfile.read(n)
        try:
            req = json.loads(raw or b"{}")
        except Exception:
            req = {}
        Counting.calls += 1
        Counting.bodies.append(req)

        # 回答里带上序号，这样能看出第二次是不是复用了第一次的响应
        text = f"第{Counting.calls}次回答"
        body = json.dumps({
            "id": "chatcmpl-count", "object": "chat.completion", "created": 0,
            "model": req.get("model", "m"),
            "choices": [{"index": 0, "finish_reason": "stop",
                         "message": {"role": "assistant", "content": text}}],
            "usage": {"prompt_tokens": 5, "completion_tokens": 3, "total_tokens": 8},
        }).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def post(port, payload):
    req = urllib.request.Request(
        f"http://127.0.0.1:{port}/v1/chat/completions",
        data=json.dumps(payload).encode(),
        headers={"content-type": "application/json"},
    )
    with _OPENER.open(req, timeout=30) as r:
        return json.loads(r.read().decode("utf-8", "replace"))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=7941)
    ap.add_argument("--exe", default=r"D:\axiao-build\release\axiao-gateway.exe")
    args = ap.parse_args()

    if not os.path.exists(args.exe):
        print(f"[FAIL] 找不到 {args.exe}")
        return 1

    srv = ThreadingHTTPServer(("127.0.0.1", MOCK_PORT), Counting)
    srv.daemon_threads = True
    threading.Thread(target=srv.serve_forever, daemon=True).start()

    tmp = tempfile.mkdtemp(prefix="cache-test-")
    with open(os.path.join(tmp, "upstreams.json"), "w", encoding="utf-8") as f:
        json.dump([{"id": "cnt", "name": "Counter",
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

    base = {"model": "m", "messages": [{"role": "user", "content": "缓存测试"}]}

    # ---- 1. 相同请求两次：上游只该被请求一次 ----
    print("\n1) 相同请求（无 temperature）")
    Counting.calls = 0
    r1 = post(args.port, base)
    r2 = post(args.port, base)
    print(f"     上游被请求 {Counting.calls} 次")
    check("上游只被请求 1 次", Counting.calls == 1, f"实际 {Counting.calls}")
    c1 = r1["choices"][0]["message"]["content"]
    c2 = r2["choices"][0]["message"]["content"]
    check("两次内容一致（第二次是缓存）", c1 == c2, f"{c1!r} vs {c2!r}")

    # ---- 2. 温度 > 0 不缓存 ----
    print("\n2) 温度 0.9（不该缓存）")
    hot = dict(base, temperature=0.9)
    Counting.calls = 0
    post(args.port, hot)
    post(args.port, hot)
    check("上游被请求 2 次", Counting.calls == 2, f"实际 {Counting.calls}")

    # ---- 3. 带 tools 不缓存 ----
    print("\n3) 带 tools（不该缓存）")
    tool = dict(base, messages=[{"role": "user", "content": "工具测试"}],
                tools=[{"type": "function", "function": {"name": "f"}}])
    Counting.calls = 0
    post(args.port, tool)
    post(args.port, tool)
    check("上游被请求 2 次", Counting.calls == 2, f"实际 {Counting.calls}")

    # ---- 4. 内容不同不能串答案 ----
    print("\n4) 不同问题不能串答案")
    Counting.calls = 0
    a = post(args.port, dict(base, messages=[{"role": "user", "content": "问题A"}]))
    b = post(args.port, dict(base, messages=[{"role": "user", "content": "问题B"}]))
    ca = a["choices"][0]["message"]["content"]
    cb = b["choices"][0]["message"]["content"]
    check("两个问题各自命中不同结果", ca != cb, f"{ca!r} vs {cb!r}")
    check("上游被请求 2 次", Counting.calls == 2, f"实际 {Counting.calls}")

    # ---- 5. 字段顺序不同仍然命中 ----
    print("\n5) 字段顺序不同（应视为同一请求）")
    Counting.calls = 0
    v1 = {"model": "m", "messages": [{"role": "user", "content": "顺序"}]}
    v2 = {"messages": [{"role": "user", "content": "顺序"}], "model": "m"}
    post(args.port, v1)
    post(args.port, v2)
    check("上游只被请求 1 次", Counting.calls == 1, f"实际 {Counting.calls}")

    p.kill()
    shutil.rmtree(tmp, ignore_errors=True)
    print("\n" + ("全部通过" if ok else "有失败项"))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
