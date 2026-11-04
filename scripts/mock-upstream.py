import json, time, random, threading
from http.server import ThreadingHTTPServer, BaseHTTPRequestHandler

class H(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    def log_message(self, *a): pass
    def do_GET(self):
        if self.path.startswith("/health") or self.path.startswith("/v1/models"):
            body = json.dumps({"ok": True, "data": [{"id": "mock-img", "object": "model"}]}).encode()
            self.send_response(200); self.send_header("content-type","application/json")
            self.send_header("content-length", str(len(body))); self.end_headers(); self.wfile.write(body)
            return
        self.send_response(404); self.send_header("content-length","0"); self.end_headers()
    def do_POST(self):
        n = int(self.headers.get("content-length") or 0)
        _ = self.rfile.read(n)
        time.sleep(random.uniform(0.15, 0.6))          # 模拟上游耗时
        if random.random() < 0.05:                      # 5% 上游抽风
            body = json.dumps({"error":{"message":"mock upstream boom"}}).encode()
            self.send_response(500)
        else:
            body = json.dumps({"created":int(time.time()),"data":[{"url":"http://127.0.0.1:18080/fake.png"}]}).encode()
            self.send_response(200)
        self.send_header("content-type","application/json")
        self.send_header("content-length", str(len(body))); self.end_headers()
        try: self.wfile.write(body)
        except Exception: pass

if __name__ == "__main__":
    srv = ThreadingHTTPServer(("127.0.0.1", 18080), H)
    srv.daemon_threads = True
    print("mock upstream on 18080", flush=True)
    srv.serve_forever()
