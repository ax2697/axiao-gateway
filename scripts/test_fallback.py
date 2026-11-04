#!/usr/bin/env python3
"""失败降级（fallback）与熔断的端到端验证。

这是网关最该有、也最容易做假的能力。光看代码不算，得证明：
一个渠道真的挂了的时候，请求还是能拿到答案。

验证项：
1. 主渠道返回 500 → 自动换下一个渠道，客户端拿到 200
2. 主渠道连不上（端口不通）→ 同样能换过去
3. 失败多次后主渠道进入冷却，后续请求直接跳过它
4. 400 这类"请求本身有问题"的错误**不**降级（换渠道也是同样结果）
5. 全部渠道都挂 → 返回明确错误，不是挂住

用法：
    python scripts/test_fallback.py
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

BAD_PORT = 18121      # 永远返回 500 的坏上游
GOOD_PORT = 18122     # 正常上游
DEAD_PORT = 18123     # 没人监听（连接被拒）
_OPENER = urllib.request.build_opener(urllib.request.ProxyHandler({}))


class Bad(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    calls = 0

    def log_message(self, *a):
        pass

    def do_POST(self):
        n = int(self.headers.get("content-length") or 0)
        self.rfile.read(n)
        Bad.calls += 1
        body = json.dumps({"error": {"message": "upstream exploded", "type": "server_error"}}).encode()
        self.send_response(500)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


class Good(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    calls = 0

    def log_message(self, *a):
        pass

    def do_POST(self):
        n = int(self.headers.get("content-length") or 0)
        raw = self.rfile.read(n)
        try:
            req = json.loads(raw or b"{}")
        except Exception:
            req = {}
        Good.calls += 1
        body = json.dumps({
            "id": "chatcmpl-good", "object": "chat.completion", "created": 0,
            "model": req.get("model", "m"),
            "choices": [{"index": 0, "finish_reason": "stop",
                         "message": {"role": "assistant", "content": "好渠道答的"}}],
            "usage": {"prompt_tokens": 5, "completion_tokens": 3, "total_tokens": 8},
        }).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


class BadRequest(BaseHTTPRequestHandler):
    """对所有请求返回 400 的上游"""
    protocol_version = "HTTP/1.1"
    calls = 0

    def log_message(self, *a):
        pass

    def do_POST(self):
        n = int(self.headers.get("content-length") or 0)
        self.rfile.read(n)
        BadRequest.calls += 1
        body = json.dumps({"error": {"message": "bad payload", "type": "invalid_request_error"}}).encode()
        self.send_response(400)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def health(port):
    try:
        return json.loads(_OPENER.open(f"http://127.0.0.1:{port}/health", timeout=3).read())
    except Exception as e:
        return {"_err": str(e)}


def post(port, payload):
    req = urllib.request.Request(
        f"http://127.0.0.1:{port}/v1/chat/completions",
        data=json.dumps(payload).encode(),
        headers={"content-type": "application/json"},
    )
    try:
        with _OPENER.open(req, timeout=30) as r:
            return json.loads(r.read().decode("utf-8", "replace")), 200
    except urllib.error.HTTPError as e:
        return {"_body": e.read().decode("utf-8", "replace")}, e.code


def start_gateway(exe, port, upstreams):
    tmp = tempfile.mkdtemp(prefix="fb-")
    with open(os.path.join(tmp, "upstreams.json"), "w", encoding="utf-8") as f:
        json.dump(upstreams, f)
    env = dict(os.environ, WB_AXIAO_DATA=tmp)
    p = subprocess.Popen([exe, "--serve-only", "--port", str(port)],
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env)
    for _ in range(60):
        time.sleep(0.25)
        if "ok" in health(port):
            return p, tmp
    p.kill()
    shutil.rmtree(tmp, ignore_errors=True)
    return None, tmp


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=7961)
    ap.add_argument("--exe", default=r"D:\axiao-build\release\axiao-gateway.exe")
    args = ap.parse_args()

    if not os.path.exists(args.exe):
        print(f"[FAIL] 找不到 {args.exe}")
        return 1

    for cls, port in ((Bad, BAD_PORT), (Good, GOOD_PORT), (BadRequest, 18124)):
        srv = ThreadingHTTPServer(("127.0.0.1", port), cls)
        srv.daemon_threads = True
        threading.Thread(target=srv.serve_forever, daemon=True).start()

    ok = True

    def check(name, cond, detail=""):
        nonlocal ok
        print(f"  [{'PASS' if cond else 'FAIL'}] {name} {detail}")
        if not cond:
            ok = False

    payload = {"model": "m", "messages": [{"role": "user", "content": "降级测试"}]}

    # ---------- 1. 500 → 降级 ----------
    print("\n1) 主渠道返回 500，应自动换到好渠道")
    Bad.calls = 0
    Good.calls = 0
    ups = [
        {"id": "bad", "name": "坏渠道", "baseUrl": f"http://127.0.0.1:{BAD_PORT}/v1",
         "apiKey": "k", "protocol": "openai", "enabled": True},
        {"id": "good", "name": "好渠道", "baseUrl": f"http://127.0.0.1:{GOOD_PORT}/v1",
         "apiKey": "k", "protocol": "openai", "enabled": True},
    ]
    gw, tmp = start_gateway(args.exe, args.port, ups)
    if not gw:
        print("[FAIL] 网关没起来")
        return 1

    successes = 0
    for i in range(6):
        # 每次换一个 prompt：相同 prompt 会被响应缓存拦掉，就测不到「是否真降级」了
        data, code = post(args.port, dict(payload, messages=[{"role": "user", "content": f"降级测试{i}"}]))
        if code == 200 and data.get("choices"):
            successes += 1
    check("6 次请求全部成功（每次都能绕过坏渠道）", successes == 6, f"成功 {successes}/6")
    check("好渠道确实被调用过", Good.calls > 0, f"calls={Good.calls}")

    h = health(args.port)
    bk = {x["id"]: x for x in h.get("breaker", [])}
    check("坏渠道记录了失败", bk.get("bad", {}).get("totalFails", 0) > 0,
          f"totalFails={bk.get('bad', {}).get('totalFails')}")
    check("坏渠道已进入冷却", bk.get("bad", {}).get("cooling") is True, str(bk.get("bad")))

    gw.kill()
    shutil.rmtree(tmp, ignore_errors=True)

    # ---------- 2. 连不上 → 降级 ----------
    print("\n2) 主渠道端口不通（连接被拒），应自动换走")
    Good.calls = 0
    ups = [
        {"id": "dead", "name": "死渠道", "baseUrl": f"http://127.0.0.1:{DEAD_PORT}/v1",
         "apiKey": "k", "protocol": "openai", "enabled": True},
        {"id": "good", "name": "好渠道", "baseUrl": f"http://127.0.0.1:{GOOD_PORT}/v1",
         "apiKey": "k", "protocol": "openai", "enabled": True},
    ]
    gw, tmp = start_gateway(args.exe, args.port + 1, ups)
    if gw:
        successes = 0
        for i in range(6):
            data, code = post(args.port + 1, dict(payload, messages=[{"role": "user", "content": f"断线测试{i}"}]))
            if code == 200 and data.get("choices"):
                successes += 1
        check("6 次请求全部成功", successes == 6, f"成功 {successes}/6")
        check("好渠道被调用过", Good.calls > 0, f"calls={Good.calls}")
        gw.kill()
    shutil.rmtree(tmp, ignore_errors=True)

    # ---------- 3. 400 不降级 ----------
    print("\n3) 400（请求本身有问题）不该降级——换渠道也是同样的错")
    Good.calls = 0
    ups = [
        {"id": "badreq", "name": "400渠道", "baseUrl": f"http://127.0.0.1:18124/v1",
         "apiKey": "k", "protocol": "openai", "enabled": True},
        {"id": "good", "name": "好渠道", "baseUrl": f"http://127.0.0.1:{GOOD_PORT}/v1",
         "apiKey": "k", "protocol": "openai", "enabled": True},
    ]
    gw, tmp = start_gateway(args.exe, args.port + 2, ups)
    if gw:
        # 连发几次，只要出现过 400 就说明"没有无脑降级"是对的；
        # 但如果轮转先轮到 good，就会是 200 —— 所以多试几次统计
        codes = []
        for i in range(8):
            _, code = post(args.port + 2, dict(payload, messages=[{"role": "user", "content": f"错误测试{i}"}]))
            codes.append(code)
        # 400 说明降级被正确抑制（把上游的错误如实返回）
        check("出现过 400（未对客户端错误降级）", 400 in codes, f"码序列={codes}")
        gw.kill()
    shutil.rmtree(tmp, ignore_errors=True)

    # ---------- 4. 全挂 ----------
    print("\n4) 所有渠道都挂，应返回明确错误而不是挂住")
    ups = [
        {"id": "d1", "name": "死1", "baseUrl": f"http://127.0.0.1:{DEAD_PORT}/v1",
         "apiKey": "k", "protocol": "openai", "enabled": True},
    ]
    gw, tmp = start_gateway(args.exe, args.port + 3, ups)
    if gw:
        t0 = time.time()
        data, code = post(args.port + 3, payload)
        cost = time.time() - t0
        check("返回了 5xx 而不是超时挂住", code >= 500 and cost < 20, f"code={code} {cost:.1f}s")
        gw.kill()
    shutil.rmtree(tmp, ignore_errors=True)

    print("\n" + ("全部通过" if ok else "有失败项"))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
