import http.server
import socketserver
import time

class MockTSHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-Type', 'video/mp2t')
        self.end_headers()
        
        # Send dummy MPEG-TS-like chunks
        chunk = b'\x47' + b'\x00' * 187 # 188 bytes starting with sync byte 0x47
        try:
            while True:
                self.wfile.write(chunk * 10) # 1.8KB
                time.sleep(0.1)
        except (ConnectionResetError, BrokenPipeError):
            print("Client disconnected")

PORT = 9999
with socketserver.TCPServer(("", PORT), MockTSHandler) as httpd:
    print(f"Mock TS Provider serving at port {PORT}")
    httpd.serve_forever()
