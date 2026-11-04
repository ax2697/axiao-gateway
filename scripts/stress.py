import json, time, statistics, threading, sys
import urllib.request, urllib.error
from concurrent.futures import ThreadPoolExecutor

OPENER = urllib.request.build_opener(urllib.request.ProxyHandler({}))  # 绕过系统代理
URL = "http://127.0.0.1:7864/v1/images/generations"
BODY = json.dumps({"prompt": "stress", "size": "1024x1024", "upstream": "mock"}).encode()

def one(i):
    t0 = time.time()
    req = urllib.request.Request(URL, data=BODY, headers={"content-type": "application/json"})
    try:
        with OPENER.open(req, timeout=60) as r:
            r.read()
            return (True, r.status, time.time() - t0, "")
    except urllib.error.HTTPError as e:
        return (False, e.code, time.time() - t0, e.read()[:120].decode("utf-8", "ignore"))
    except Exception as e:
        return (False, 0, time.time() - t0, str(e)[:120])

def run(conc, total):
    with ThreadPoolExecutor(max_workers=conc) as ex:
        res = list(ex.map(one, range(total)))
    ok = [r for r in res if r[0]]
    lat = sorted(r[2] for r in res)
    codes = {}
    for r in res:
        codes[r[1]] = codes.get(r[1], 0) + 1
    print(f"并发 {conc:>2} / 共 {total:>3} 请求 → 成功 {len(ok):>3} ({len(ok)*100//total}%)  状态码 {codes}")
    if lat:
        print(f"        延迟 p50={lat[len(lat)//2]:.2f}s  p95={lat[int(len(lat)*0.95)-1]:.2f}s  max={lat[-1]:.2f}s")
    errs = [r[3] for r in res if not r[0] and r[1] not in (500,)]
    if errs:
        print("        非 500 错误样例:", errs[0])
    return len(ok), total

if __name__ == "__main__":
    # 预热
    one(0)
    for conc in (1, 5, 20, 50):
        run(conc, 40 if conc <= 20 else 80)
