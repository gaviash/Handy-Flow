# Fake OpenAI-compatible endpoint for the context bench: records every request
# body (what Handy would send to Groq) and answers with the dictation itself.
# Usage: python -I mock_llm.py <port> <log.jsonl>
import json, re, sys
from http.server import BaseHTTPRequestHandler, HTTPServer

LOG = sys.argv[2]

class H(BaseHTTPRequestHandler):
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        body['messages'] = [m for m in body['messages'] if m['role'] != 'system']
        with open(LOG, 'a', encoding='utf-8') as f:
            f.write(json.dumps(body, ensure_ascii=False) + '\n')
        user = body['messages'][-1]['content']
        m = re.search(r'<dictation>\n(.*)\n</dictation>', user, re.S)
        t = re.search(r'<text>\n(.*)\n</text>', user, re.S)
        # A transformation answers with a visibly changed text.
        content = '[transformé] ' + t.group(1) if t else (m.group(1) if m else user)
        reply = {'choices': [{'message': {'role': 'assistant', 'content': content}}]}
        data = json.dumps(reply).encode()
        self.send_response(200); self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(data))); self.end_headers(); self.wfile.write(data)
    def log_message(self, *a): pass

HTTPServer(('127.0.0.1', int(sys.argv[1])), H).serve_forever()
