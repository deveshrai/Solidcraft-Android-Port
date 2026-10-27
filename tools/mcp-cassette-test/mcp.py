"""A small MCP client for SolidCraft's MCP server over HTTP that times every call:
init(), then tool(name, args) or ex(command, params). MCP_URL, MCP_TOKEN and MCP_LOG set the
server, its access token and the timings file (JSON lines)."""
import json, time, urllib.request, os, sys
URL = os.environ.get("MCP_URL", "http://127.0.0.1:27880/mcp")
TOKEN = os.environ["MCP_TOKEN"]
LOG = os.environ.get("MCP_LOG", os.path.join(os.path.dirname(__file__), "timings.jsonl"))
_sid = None
_id = 0
def _post(msg, timeout=300):
    global _sid
    h = {"Authorization": "Bearer " + TOKEN, "Content-Type": "application/json", "Accept": "application/json, text/event-stream"}
    if _sid: h["Mcp-Session-Id"] = _sid
    req = urllib.request.Request(URL, data=json.dumps(msg).encode(), headers=h, method="POST")
    with urllib.request.urlopen(req, timeout=timeout) as r:
        if r.headers.get("Mcp-Session-Id"): _sid = r.headers["Mcp-Session-Id"]
        body = r.read().decode()
        return json.loads(body) if body else None
def init():
    global _id
    _id += 1
    r = _post({"jsonrpc": "2.0", "id": _id, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "cassette-test", "version": "1"}}})
    _post({"jsonrpc": "2.0", "method": "notifications/initialized"})
    return r
def tool(name, args=None, label=None, timeout=300, quiet=False):
    global _id
    _id += 1
    t0 = time.time()
    err = None
    try:
        r = _post({"jsonrpc": "2.0", "id": _id, "method": "tools/call", "params": {"name": name, "arguments": args or {}}}, timeout)
    except Exception as e:
        r = None; err = f"TRANSPORT: {e}"
    dt = time.time() - t0
    res = (r or {}).get("result", {})
    text = "".join(c.get("text", "") for c in res.get("content", []) if c.get("type") == "text")
    if r and "error" in r: err = r["error"].get("message")
    if res.get("isError"): err = text
    rec = {"label": label or (args or {}).get("command", name), "tool": name, "t0": round(t0, 3), "s": round(dt, 3), "ok": err is None, "error": (err or "")[:400]}
    with open(LOG, "a") as f: f.write(json.dumps(rec) + "\n")
    if not quiet: print(f"[{dt:7.2f}s] {'OK ' if err is None else 'ERR'} {rec['label']}" + (f"  -> {err[:300]}" if err else ""), flush=True)
    if err: raise RuntimeError(f"{rec['label']}: {err}")
    imgs = [c for c in res.get("content", []) if c.get("type") == "image"]
    try:
        out = json.loads(text)
        if isinstance(out, dict) and imgs: out["_images"] = imgs
        return out
    except Exception: return {"_text": text, "_images": imgs}
def ex(command, params=None, label=None, **kw):
    return tool("execute", {"command": command, "params": params or {}}, label=label or command, **kw)
