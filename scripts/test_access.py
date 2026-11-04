#!/usr/bin/env python3
"""访问 Key 的端到端验证。

这是"把网关给别人用"的前置条件。之前只有一个字符串数组，没有任何约束——
发出去就能无限刷你的上游额度。

验证项：
1. 没配 Key → 全开（保持便携版开箱即用）
2. 配了 Key → 不带/带错都 401
3. 带对的 Key → 200，并计入今日用量
4. 每日上限 → 超了 429，且提示里说清用了多少
5. 停用的 Key → 401
6. 流式通道同样校验

用法：
    python scripts/test_access.py
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

MOCK_PORT = 18151
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
            self.wfile.write(b'data: {"choices":[{"delta":{"content":"ok"}}]}\n\n')
            self.wfile.write(b"data: [DONE]\n\n")
            self.wfile.flush()
            return
        body = json.dumps({
            "id": "c", "object": "chat.completion", "created": 0,
            "model": req.get("model", "m"),
            "choices": [{"index": 0, "finish_reason": "stop",
                         "message": {"role": "assistant", "content": "好"}}],
            "usage": {"prompt_tokens": 3, "completion_tokens": 2, "total_tokens": 5},
        }).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def start(exe, port, keys):
    tmp = tempfile.mkdtemp(prefix="acc-")
    with open(os.path.join(tmp, "upstreams.json"), "w", encoding="utf-8") as f:
        json.dump([{"id": "m", "name": "M", "baseUrl": f"http://127.0.0.1:{MOCK_PORT}/v1",
                    "apiKey": "up", "protocol": "openai", "enabled": True}], f)
    with open(os.path.join(tmp, "extras.json"), "w", encoding="utf-8") as f:
        json.dump({"control": {"keys": keys}}, f)
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


def call(port, key=None, stream=False):
    headers = {"content-type": "application/json"}
    if key:
        headers["authorization"] = f"Bearer {key}"
    payload = {"model": "m", "messages": [{"role": "user", "content": "hi"}]}
    if stream:
        payload["stream"] = True
    req = urllib.request.Request(f"http://127.0.0.1:{port}/v1/chat/completions",
                                data=json.dumps(payload).encode(), headers=headers)
    try:
        with _OPENER.open(req, timeout=30) as r:
            return r.status if hasattr(r, "status") else 200, r.read().decode("utf-8", "replace")
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode("utf-8", "replace")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=7991)
    ap.add_argument("--exe", default=r"D:\axiao-build\release\axiao-gateway.exe")
    args = ap.parse_args()
    if not os.path.exists(args.exe):
        print(f"[FAIL] 找不到 {args.exe}")
        return 1

    srv = ThreadingHTTPServer(("127.0.0.1", MOCK_PORT), Mock)
    srv.daemon_threads = True
    threading.Thread(target=srv.serve_forever, daemon=True).start()

    ok = True

    def check(name, cond, detail=""):
        nonlocal ok
        print(f"  [{'PASS' if cond else 'FAIL'}] {name} {detail}")
        if not cond:
            ok = False

    # ---- 1. 没配 Key：全开 ----
    print("\n1) 没配任何 Key：全开（便携版开箱即用）")
    gw, tmp = start(args.exe, args.port, [])
    if not gw:
        print("[FAIL] 网关没起来")
        return 1
    c, _ = call(args.port)
    check("不带 Key 也能调", c == 200, f"code={c}")
    try:
        h = json.loads(_OPENER.open(f"http://127.0.0.1:{args.port}/health", timeout=3).read())
        check("/health 报 authRequired=false", h.get("authRequired") is False, str(h.get("authRequired")))
    except Exception as e:
        check("/health 报 authRequired=false", False, str(e))
    gw.kill(); shutil.rmtree(tmp, ignore_errors=True)

    # ---- 2. 配 Key：必须带 ----
    print("\n2) 配了 Key：不带 / 带错都该 401")
    keys = [{"id": "k1", "name": "给老王的", "key": "sk-gw-testkey0001",
             "dailyLimit": 2, "enabled": True}]
    gw, tmp = start(args.exe, args.port + 1, keys)
    if gw:
        c, body = call(args.port + 1)
        check("不带 Key → 401", c == 401, f"code={c}")
        c, _ = call(args.port + 1, key="sk-wrong")
        check("带错 Key → 401", c == 401, f"code={c}")

        c1, _ = call(args.port + 1, key="sk-gw-testkey0001")
        check("带对的 Key → 200", c1 == 200, f"code={c1}")
        c2, _ = call(args.port + 1, key="sk-gw-testkey0001")
        check("第 2 次仍然 200（上限是 2）", c2 == 200, f"code={c2}")
        c3, body3 = call(args.port + 1, key="sk-gw-testkey0001")
        check("第 3 次 → 429（超每日上限）", c3 == 429, f"code={c3}")
        check("429 里说清了用量", "上限" in body3 or "limit" in body3.lower(), body3[:100])

        # 流式通道也该校验
        c4, _ = call(args.port + 1, key="sk-wrong", stream=True)
        check("流式通道同样拦错误 Key", c4 == 401, f"code={c4}")
        gw.kill()
    shutil.rmtree(tmp, ignore_errors=True)

    # ---- 2.5 每分钟限速 ----
    print("\n2.5) 每分钟限速（挡短时间猛刷）")
    keys = [{"id": "k3", "name": "限速的", "key": "sk-gw-rpmtest0001",
             "dailyLimit": None, "rpmLimit": 2, "enabled": True}]
    gw, tmp = start(args.exe, args.port + 4, keys)
    if gw:
        r1, _ = call(args.port + 4, key="sk-gw-rpmtest0001")
        r2, _ = call(args.port + 4, key="sk-gw-rpmtest0001")
        r3, body3 = call(args.port + 4, key="sk-gw-rpmtest0001")
        check("前 2 次通过（上限 2 次/分）", r1 == 200 and r2 == 200, f"{r1},{r2}")
        check("第 3 次 429", r3 == 429, f"code={r3}")
        check("429 提示里说明是按分钟限的", "分钟" in body3 or "分" in body3, body3[:110])
        gw.kill()
    shutil.rmtree(tmp, ignore_errors=True)

    # ---- 3. 停用 ----
    print("\n3) 停用的 Key 不该能用")
    keys = [{"id": "k2", "name": "停用的", "key": "sk-gw-disabled01",
             "dailyLimit": None, "enabled": False}]
    gw, tmp = start(args.exe, args.port + 2, keys)
    if gw:
        c, _ = call(args.port + 2, key="sk-gw-disabled01")
        check("停用的 Key → 401", c == 401, f"code={c}")
        gw.kill()
    shutil.rmtree(tmp, ignore_errors=True)

    # ---- 4. 老格式兼容 ----
    print("\n4) 老配置（纯字符串数组）仍能识别")
    gw, tmp = start(args.exe, args.port + 3, ["sk-legacy-key-001"])
    if gw:
        c, _ = call(args.port + 3, key="sk-legacy-key-001")
        check("老格式的 Key 能用", c == 200, f"code={c}")
        c2, _ = call(args.port + 3)
        check("老格式下不带 Key 也拦", c2 == 401, f"code={c2}")
        gw.kill()
    shutil.rmtree(tmp, ignore_errors=True)

    print("\n" + ("全部通过" if ok else "有失败项"))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
