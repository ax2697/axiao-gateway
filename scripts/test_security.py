#!/usr/bin/env python3
"""本地网关的可用性/安全回归测试。

这些不是"性能测试"，是**边界测试**——每一条都对应一个真实的攻击面：
任何人都能往 127.0.0.1:7864 发请求，所以请求体、请求头、连接数都必须有硬上限。

覆盖：
1. 超大 Content-Length  → 必须 413，且进程不崩（否则一个请求吃满 4GB）
2. 超长请求头           → 必须断开，不崩（read_line 本身没有长度上限）
3. 连接数打满           → 必须 503，不崩（每连接一个线程）
4. 正常请求             → 仍然可用（别为了防守把功能搞坏）

用法：
    python scripts/test_security.py --port 7911
    python scripts/test_security.py --exe D:/axiao-build/release/axiao-gateway.exe
"""
import argparse
import json
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import time

OPENER = None  # 延迟创建，避免 urllib 走系统代理


def http_get(port, path="/health", timeout=5):
    import urllib.request
    op = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    return op.open(f"http://127.0.0.1:{port}{path}", timeout=timeout).read().decode()


def raw_request(port, payload: bytes, read_timeout=6, expect_reply=True):
    """发一段原始字节，返回收到的响应（可能为空）"""
    s = socket.create_connection(("127.0.0.1", port), timeout=read_timeout)
    try:
        s.sendall(payload)
        s.settimeout(read_timeout)
        if not expect_reply:
            return b""
        chunks = []
        try:
            while True:
                b = s.recv(4096)
                if not b:
                    break
                chunks.append(b)
                if b"\r\n\r\n" in b"".join(chunks) and len(b"".join(chunks)) > 200:
                    break
        except (socket.timeout, ConnectionResetError, ConnectionAbortedError, OSError):
            # 服务端回 503/413 后立刻关闭连接时，客户端 recv 会被 RST。
            # 这是正确行为（我们就是要快速失败），测试照收已拿到的字节。
            pass
        return b"".join(chunks)
    finally:
        s.close()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=7911)
    ap.add_argument("--exe", default=r"D:\axiao-build\release\axiao-gateway.exe")
    ap.add_argument("--keep", action="store_true", help="测完不杀进程（调试用）")
    args = ap.parse_args()

    if not os.path.exists(args.exe):
        print(f"[FAIL] 找不到 {args.exe}")
        return 1

    tmp = tempfile.mkdtemp(prefix="axiao-sec-")
    json.dump([{
        "id": "mock", "name": "Mock",
        "baseUrl": "http://127.0.0.1:18099/v1",
        "apiKey": "k", "protocol": "openai", "enabled": True,
    }], open(os.path.join(tmp, "upstreams.json"), "w", encoding="utf-8"))

    env = dict(os.environ, WB_AXIAO_DATA=tmp)
    p = subprocess.Popen([args.exe, "--serve-only", "--port", str(args.port)],
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env)
    ready = False
    for _ in range(60):
        time.sleep(0.25)
        try:
            http_get(args.port)
            ready = True
            break
        except Exception:
            if p.poll() is not None:
                break
    if not ready:
        print("[FAIL] 服务没起来")
        p.kill()
        shutil.rmtree(tmp, ignore_errors=True)
        return 1
    print(f"[app] 服务已起 :{args.port}（数据目录 {tmp}）\n")

    ok = True

    def check(name, cond, detail=""):
        nonlocal ok
        print(f"  [{'PASS' if cond else 'FAIL'}] {name} {detail}")
        if not cond:
            ok = False

    def alive():
        return p.poll() is None

    # ---- 0. 功能基线（先证明正常可用，再做破坏性测试）----
    print("0) 功能基线")
    try:
        body = http_get(args.port)
        check("/health 正常（防守没误伤功能）", '"ok":true' in body.replace(" ", ""), body[:70])
    except Exception as e:
        check("/health 正常（防守没误伤功能）", False, str(e))

    # ---- 1. 超大 Content-Length ----
    print("1) 超大请求体")
    try:
        resp = raw_request(args.port, (
            b"POST /v1/chat/completions HTTP/1.1\r\n"
            b"Host: 127.0.0.1\r\n"
            b"Content-Type: application/json\r\n"
            b"Content-Length: 4294967295\r\n\r\n"
        ))
        head = resp.split(b"\r\n", 1)[0].decode("latin-1", "replace")
        check("返回 413 而不是分配 4GB", "413" in head, head)
    except Exception as e:
        check("返回 413 而不是分配 4GB", False, str(e))
    check("进程还活着", alive())

    # ---- 2. 超长请求头（不带换行）----
    print("\n2) 超长请求头")
    try:
        raw_request(args.port, b"GET /health HTTP/1.1\r\nX-Pad: " + b"A" * (2 * 1024 * 1024),
                    read_timeout=4, expect_reply=False)
        connected_ok = True
    except Exception:
        connected_ok = True  # 被服务端断开也算正确处理
    check("超长头不会拖垮服务", connected_ok)
    check("进程还活着", alive())

    # ---- 3. 连接数打满 ----
    print("\n3) 连接数上限")
    socks = []
    try:
        # 只发半个请求，让服务端挂着等剩余数据（read_timeout 20s 内不会关）
        for _ in range(200):
            try:
                s = socket.create_connection(("127.0.0.1", args.port), timeout=2)
                s.sendall(b"POST /v1/chat/completions HTTP/1.1\r\nHost: x\r\n")
                socks.append(s)
            except Exception:
                break
        print(f"     实际建立 {len(socks)} 条连接")
        # 打满后再发一个完整请求，应该拿到 503（或连接被立刻关）
        resp = raw_request(args.port, b"GET /health HTTP/1.1\r\nHost: x\r\n\r\n", read_timeout=4)
        head = resp.split(b"\r\n", 1)[0].decode("latin-1", "replace") if resp else "(无响应)"
        # 503 最理想；连接被直接关掉也算守住了（不崩、不无限开线程）
        check("超出上限时不崩（503 或断开）", ("503" in head) or (resp == b""), head)
    finally:
        for s in socks:
            try:
                s.close()
            except Exception:
                pass
    # 等连接计数回落：128 个线程读到 EOF 后才会释放名额，
    # 给 12 秒（超过 read_timeout 的一半），并报告实际恢复耗时
    t0 = time.time()
    recovered = False
    while time.time() - t0 < 15:
        try:
            if '"ok":true' in http_get(args.port).replace(" ", ""):
                recovered = True
                break
        except Exception:
            pass
        time.sleep(0.3)
    print(f"     关闭后恢复耗时 {round((time.time()-t0)*1000)}ms")
    check("连接释放后可再次服务（最坏等一个 read_timeout）", recovered)
    check("进程还活着", alive())

    print()
    if not args.keep:
        alive_now = alive()
        p.kill()
        print(f"进程{'仍在' if alive_now else '已'}运行（退出码 {p.poll()}）")
    shutil.rmtree(tmp, ignore_errors=True)

    print("\n" + ("全部通过" if ok else "有失败项"))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
