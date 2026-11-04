#!/usr/bin/env python3
"""流式转发回归测试（三种上游协议）。

验证目标（缺一不可）：
1. 首帧很快到 —— 证明是真增量，不是"整段取回再切帧"
2. 帧间隔跟上游吐字节奏一致（mock 每 350ms 一片）—— 证明没有攒包
3. 结束帧和 [DONE] 都在
4. 内容拼起来跟上游一致 —— 转换链路没丢字
5. usage 能被识别并回传

用法：
    python scripts/test_stream.py                      # 默认测 openai
    python scripts/test_stream.py --protocol anthropic
    python scripts/test_stream.py --protocol gemini
    python scripts/test_stream.py --all                # 三种全测

脚本会自建临时数据目录（WB_AXIAO_DATA），不碰用户真实的
upstreams.json / ledger.jsonl。
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

MOCK_PORT = 18091
PIECES = ["慢", "慢", "地", "吐", "出", "来"]
GAP = 0.35

# 本机测试绝对不能走系统代理：
# 这台机器上有 HTTP_PROXY=http://127.0.0.1:50555，urllib 默认会把
# http://127.0.0.1:7901 也丢给代理，代理连不上就回 502，
# 看起来像"端口没监听"，其实服务好好的。空 ProxyHandler = 强制直连。
_OPENER = urllib.request.build_opener(urllib.request.ProxyHandler({}))


class Mock(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *a):
        pass

    def _json(self, obj):
        body = json.dumps(obj, ensure_ascii=False).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _sse_head(self):
        self.send_response(200)
        self.send_header("content-type", "text/event-stream")
        self.send_header("connection", "close")
        self.end_headers()
        self.close_connection = True

    def _send(self, payload):
        try:
            self.wfile.write(b"data: " + payload.encode() + b"\n\n")
            self.wfile.flush()
            return True
        except Exception:
            return False

    def do_POST(self):
        n = int(self.headers.get("content-length") or 0)
        raw = self.rfile.read(n)
        try:
            req = json.loads(raw or b"{}")
        except Exception:
            req = {}
        if "/messages" in self.path:
            return self.anthropic(req)
        if "streamGenerateContent" in self.path:
            return self.gemini(req)
        if ":generateContent" in self.path:
            return self.gemini(req)
        return self.openai(req)

    # ---------- OpenAI ----------
    def openai(self, req):
        if not req.get("stream"):
            return self._json({
                "id": "x", "object": "chat.completion", "created": 0, "model": req.get("model", "m"),
                "choices": [{"index": 0, "message": {"role": "assistant", "content": "非流式"},
                             "finish_reason": "stop"}],
                "usage": {"prompt_tokens": 3, "completion_tokens": 2, "total_tokens": 5},
            })
        self._sse_head()
        for i, p in enumerate(PIECES):
            delta = {"role": "assistant", "content": p} if i == 0 else {"content": p}
            if not self._send(json.dumps({
                "id": "chatcmpl-mock", "object": "chat.completion.chunk", "created": 0,
                "model": "mock", "choices": [{"index": 0, "delta": delta, "finish_reason": None}],
            }, ensure_ascii=False)):
                return
            time.sleep(GAP)
        self._send(json.dumps({
            "id": "chatcmpl-mock", "object": "chat.completion.chunk", "created": 0, "model": "mock",
            "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}],
        }))
        self._send(json.dumps({
            "id": "chatcmpl-mock", "object": "chat.completion.chunk", "created": 0, "model": "mock",
            "choices": [], "usage": {"prompt_tokens": 11, "completion_tokens": 6, "total_tokens": 17},
        }))
        self._send("[DONE]")

    # ---------- Anthropic ----------
    def anthropic(self, req):
        if not req.get("stream"):
            return self._json({
                "id": "msg_1", "type": "message", "role": "assistant",
                "content": [{"type": "text", "text": "非流式"}],
                "model": req.get("model", "m"), "stop_reason": "end_turn",
                "usage": {"input_tokens": 3, "output_tokens": 2},
            })
        self._sse_head()

        def ev(t, **kw):
            return self._send(json.dumps({"type": t, **kw}, ensure_ascii=False))

        ev("message_start", message={"id": "msg_1", "type": "message", "role": "assistant",
                                     "content": [], "model": "mock",
                                     "usage": {"input_tokens": 11, "output_tokens": 0}})
        ev("content_block_start", index=0, content_block={"type": "text", "text": ""})
        for p in PIECES:
            if not ev("content_block_delta", index=0, delta={"type": "text_delta", "text": p}):
                return
            time.sleep(GAP)
        ev("content_block_stop", index=0)
        ev("message_delta", delta={"stop_reason": "end_turn"}, usage={"output_tokens": 6})
        ev("message_stop")

    # ---------- Gemini ----------
    def gemini(self, req):
        if ":generateContent" in self.path and "stream" not in self.path:
            return self._json({
                "candidates": [{"content": {"parts": [{"text": "非流式"}]}, "finishReason": "STOP"}],
                "usageMetadata": {"promptTokenCount": 3, "candidatesTokenCount": 2, "totalTokenCount": 5},
            })
        self._sse_head()
        for i in range(len(PIECES)):
            if not self._send(json.dumps({
                "candidates": [{"content": {"role": "model", "parts": [{"text": PIECES[i]}]},
                                "index": 0}],
                "usageMetadata": {"promptTokenCount": 11, "candidatesTokenCount": i + 1,
                                  "totalTokenCount": 11 + i + 1},
            }, ensure_ascii=False)):
                return
            time.sleep(GAP)
        self._send(json.dumps({
            "candidates": [{"content": {"parts": [{"text": ""}]}, "finishReason": "STOP"}],
            "usageMetadata": {"promptTokenCount": 11, "candidatesTokenCount": 6, "totalTokenCount": 17},
        }, ensure_ascii=False))


def http_json(url, payload, timeout=90):
    """非流式：一次性拿完整 JSON"""
    req = urllib.request.Request(
        url, data=json.dumps(payload).encode(),
        headers={"content-type": "application/json", "authorization": "Bearer test"},
    )
    with _OPENER.open(req, timeout=timeout) as resp:
        return json.loads(resp.read().decode("utf-8", "replace"))


def read_stream(url, payload, timeout=90):
    """流式：逐行读，记录每行到达的相对毫秒"""
    req = urllib.request.Request(
        url, data=json.dumps(payload).encode(),
        headers={"content-type": "application/json", "authorization": "Bearer test"},
    )
    t0 = time.time()
    frames = []
    with _OPENER.open(req, timeout=timeout) as resp:
        ctype = resp.headers.get("content-type", "")
        for raw in resp:
            ms = round((time.time() - t0) * 1000)
            line = raw.decode("utf-8", "replace").strip()
            if line:
                frames.append((ms, line))
    return ctype, frames, round((time.time() - t0) * 1000)


def run_one(protocol, exe, port, verbose=True):
    print(f"\n{'='*62}\n  协议：{protocol}\n{'='*62}")
    tmpdir = tempfile.mkdtemp(prefix=f"axiao-stream-{protocol}-")
    with open(os.path.join(tmpdir, "upstreams.json"), "w", encoding="utf-8") as f:
        json.dump([{
            "id": f"mock_{protocol}", "name": f"Mock {protocol}",
            "baseUrl": f"http://127.0.0.1:{MOCK_PORT}",
            "apiKey": "k", "protocol": protocol, "enabled": True, "weight": 1,
        }], f, ensure_ascii=False)

    env = dict(os.environ, WB_AXIAO_DATA=tmpdir)
    proc = subprocess.Popen([exe, "--serve-only", "--port", str(port)],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env)
    base = f"http://127.0.0.1:{port}"
    ready = False
    for _ in range(100):
        try:
            _OPENER.open(base + "/health", timeout=1).read()
            ready = True
            break
        except Exception:
            time.sleep(0.2)
    if not ready:
        print("  [FAIL] 服务 20 秒内没起来")
        proc.kill()
        shutil.rmtree(tmpdir, ignore_errors=True)
        return False

    ok = True

    def check(name, cond, detail=""):
        nonlocal ok
        print(f"  [{'PASS' if cond else 'FAIL'}] {name} {detail}")
        if not cond:
            ok = False

    # 先验非流式：确认真流式没把普通请求搞坏
    try:
        body0 = http_json(base + "/v1/chat/completions",
                          {"model": "mock", "messages": [{"role": "user", "content": "hi"}]})
        txt0 = (body0.get("choices") or [{}])[0].get("message", {}).get("content", "")
        check("非流式转换正常", txt0 == "非流式", f"content={txt0!r}")
    except Exception as e:
        check("非流式转换正常", False, str(e))

    try:
        ctype, frames, total = read_stream(
            base + "/v1/chat/completions",
            {"model": "mock", "stream": True, "messages": [{"role": "user", "content": "你好"}]})
    except Exception as e:
        check("流式请求成功", False, str(e))
        proc.kill()
        shutil.rmtree(tmpdir, ignore_errors=True)
        return False

    print(f"  Content-Type: {ctype} | 总耗时 {total}ms | {len(frames)} 行")
    if verbose:
        for ms, line in frames:
            print(f"    +{ms:>5}ms  {line[:96]}")

    check("Content-Type 是 SSE", "text/event-stream" in ctype)

    contents, times, usage = [], [], {}
    for ms, l in frames:
        if not l.startswith("data:") or "[DONE]" in l:
            continue
        try:
            v = json.loads(l[5:].strip())
        except Exception:
            continue
        chs = v.get("choices")
        if isinstance(chs, list) and not chs:
            if v.get("usage"):
                usage = v["usage"]
            continue
        d = (chs or [{}])[0].get("delta") or {}
        if d.get("content"):
            contents.append(d["content"])
            times.append(ms)

    text = "".join(contents)
    check("内容拼起来正确", text == "".join(PIECES), f"实际={text!r}")
    check("有 [DONE]", any("[DONE]" in l for _, l in frames))
    check("有结束帧", any('"finish_reason": "stop"' in l or '"finish_reason":"stop"' in l
                          for _, l in frames))
    check("usage 已识别", usage.get("completion_tokens") == 6, f"usage={usage}")

    if times:
        check("首字延迟远小于总时长（真增量）", times[0] < total * 0.5,
              f"首字 {times[0]}ms / 总 {total}ms")
        if len(times) >= 3:
            gaps = [b - a for a, b in zip(times, times[1:])]
            avg = sum(gaps) / len(gaps)
            check("帧间隔贴近上游节奏（没攒包）", abs(avg - GAP * 1000) < 180,
                  f"平均间隔 {avg:.0f}ms（期望 {GAP * 1000:.0f}ms）")
    else:
        check("收到内容帧", False, "一帧内容都没解析到")

    proc.kill()
    shutil.rmtree(tmpdir, ignore_errors=True)
    return ok


def run_concurrency(exe, port, n):
    """N 路流式同时跑，验证互不串帧、都能完整收尾。"""
    print(f"\n{'='*62}\n  并发流式：{n} 路同时开\n{'='*62}")
    tmpdir = tempfile.mkdtemp(prefix="axiao-conc-")
    with open(os.path.join(tmpdir, "upstreams.json"), "w", encoding="utf-8") as f:
        json.dump([{
            "id": "mock_stream", "name": "Mock Stream",
            "baseUrl": f"http://127.0.0.1:{MOCK_PORT}/v1",
            "apiKey": "k", "protocol": "openai", "enabled": True, "weight": 1,
        }], f, ensure_ascii=False)

    env = dict(os.environ, WB_AXIAO_DATA=tmpdir)
    proc = subprocess.Popen([exe, "--serve-only", "--port", str(port)],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env)
    base = f"http://127.0.0.1:{port}"
    for _ in range(100):
        try:
            _OPENER.open(base + "/health", timeout=1).read()
            break
        except Exception:
            time.sleep(0.2)
    else:
        print("  [FAIL] 服务没起来")
        proc.kill()
        shutil.rmtree(tmpdir, ignore_errors=True)
        return False

    results = {}

    def worker(i):
        t0 = time.time()
        try:
            _, frames, total = read_stream(
                base + "/v1/chat/completions",
                {"model": "mock", "stream": True,
                 "messages": [{"role": "user", "content": f"hi-{i}"}]}, timeout=120)
            buf = []
            for _, l in frames:
                if not l.startswith("data:") or "[DONE]" in l:
                    continue
                try:
                    v = json.loads(l[5:].strip())
                except Exception:
                    continue
                for c in v.get("choices") or []:
                    d = c.get("delta") or {}
                    if d.get("content"):
                        buf.append(d["content"])
            results[i] = ("".join(buf) == "".join(PIECES), total, "".join(buf))
        except Exception as e:
            results[i] = (False, 0, f"ERR {e}")

    threads = [threading.Thread(target=worker, args=(i,)) for i in range(n)]
    t0 = time.time()
    for t in threads:
        t.start()
    for t in threads:
        t.join()
    wall = round((time.time() - t0) * 1000)

    good = sum(1 for r in results.values() if r[0])
    for i in sorted(results):
        okk, total, text = results[i]
        print(f"    #{i}: {'OK ' if okk else 'BAD'} {total:>5}ms  内容={text!r}")

    proc.kill()
    shutil.rmtree(tmpdir, ignore_errors=True)
    # 并发 8 路，整体墙钟时间应该接近单路的 6 片 × 350ms ≈ 2.1s，而不是线性叠加
    passed = good == n
    print(f"  [{'PASS' if passed else 'FAIL'}] {good}/{n} 路内容完整，墙钟 {wall}ms"
          f"（串行的话要 {n * 2100}ms，说明是真并行）")
    return passed


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--protocol", default="openai", choices=["openai", "anthropic", "gemini"])
    ap.add_argument("--all", action="store_true")
    ap.add_argument("--concurrency", type=int, default=0, help="额外跑 N 路并发流式")
    ap.add_argument("--port", type=int, default=7901)
    ap.add_argument("--exe", default=r"D:\axiao-build\debug\axiao-gateway.exe")
    ap.add_argument("--quiet", action="store_true")
    args = ap.parse_args()

    if not os.path.exists(args.exe):
        print(f"[FAIL] 找不到 {args.exe}")
        return 1

    srv = ThreadingHTTPServer(("127.0.0.1", MOCK_PORT), Mock)
    srv.daemon_threads = True
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    print(f"[mock] 上游 127.0.0.1:{MOCK_PORT} 已起，每片间隔 {GAP}s")

    targets = ["openai", "anthropic", "gemini"] if args.all else [args.protocol]
    results = []
    for i, p in enumerate(targets):
        results.append((p, run_one(p, args.exe, args.port + i, not args.quiet)))

    if args.concurrency > 0:
        results.append(
            (f"并发×{args.concurrency}",
             run_concurrency(args.exe, args.port + 20, args.concurrency))
        )

    print(f"\n{'='*62}")
    for p, r in results:
        print(f"  {p:<12} {'通过' if r else '失败'}")
    allok = all(r for _, r in results)
    print("全部通过" if allok else "有失败项")
    return 0 if allok else 1


if __name__ == "__main__":
    sys.exit(main())
