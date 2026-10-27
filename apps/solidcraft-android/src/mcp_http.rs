//! The MCP server on the network (Android port): MCP's "Streamable HTTP" transport, answering
//! each POST with JSON (no event stream), so Claude Code in Termux, or on a computer on the same
//! Wi-Fi, can drive the app.
//!
//! Each MCP session is upstream's own [`solidcraft_mcp::Server`], made by the host (bridged to
//! the app's loopback control channel, as `solidcraft-cli mcp` is), so the tools are exactly the
//! desktop's. This adds only HTTP and a bearer token: every request must carry
//! `Authorization: Bearer <token>`. Built and tested on every system; only the app runs it.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Longest request line or header block accepted.
const MAX_HEADERS: usize = 16 << 10;
/// Longest request body: the MCP server's own limit on a message.
const MAX_BODY: usize = solidcraft_engine::MAX_LINE;
/// Sessions kept at once; the one used longest ago goes first.
const MAX_SESSIONS: usize = 8;
/// How long a client may take to send its request.
const READ_TIMEOUT: Duration = Duration::from_secs(30);
/// How often the listener looks whether it should stop.
const POLL: Duration = Duration::from_millis(50);

/// Makes the MCP server for a new session (on the endpoint's thread).
pub type MakeServer = Box<dyn Fn() -> solidcraft_mcp::Server + Send>;
/// Whether the app can run a tool call now (Android pauses it off screen).
pub type Ready = Box<dyn Fn() -> bool + Send>;

/// How long a tool call waits for the app to come back on screen before it is answered.
const READY_WAIT: Duration = Duration::from_secs(15);
/// What a tool call that couldn't run says, for the agent to pass on.
const NOT_ON_SCREEN: &str = "SolidCraft is in the background on the Android device, and Android pauses it there, so this call did \
not run. Ask the user to bring SolidCraft back on screen (split screen or a pop-up window keeps it running next to Termux), \
then send the call again.";

/// What the endpoint has done, for the settings page.
#[derive(Clone, Debug, Default)]
pub struct Activity {
    pub sessions: usize,
    pub requests: u64,
    pub last_request: Option<Instant>,
    /// Requests refused for a missing or wrong token.
    pub refused: u64,
}

/// A running endpoint. Dropping it stops it.
pub struct Endpoint {
    port: u16,
    stop: Arc<AtomicBool>,
    activity: Arc<Mutex<Activity>>,
    thread: Option<JoinHandle<()>>,
}

impl Endpoint {
    /// Listen on `addr` (port 0: any free one) for clients with `token`.
    pub fn start(addr: SocketAddr, token: String, make: MakeServer, ready: Ready) -> std::io::Result<Endpoint> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let stop = Arc::new(AtomicBool::new(false));
        let activity = Arc::new(Mutex::new(Activity::default()));
        let (s, a) = (stop.clone(), activity.clone());
        let thread = std::thread::Builder::new().name("mcp-http".into()).spawn(move || serve(listener, &token, &make, &ready, &s, &a))?;
        Ok(Endpoint { port, stop, activity, thread: Some(thread) })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn activity(&self) -> Activity {
        self.activity.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// Stop listening. Waits a moment for the thread to let go of the port, so a new endpoint can
    /// take it; a tool call still running finishes on its own.
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let deadline = Instant::now() + Duration::from_secs(2);
            while !thread.is_finished() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

impl Drop for Endpoint {
    fn drop(&mut self) {
        self.stop();
    }
}

/// A new random token: 32 bytes from the system, as hex.
pub fn new_token() -> Option<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).ok()?;
    Some(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// This device's address on its network (the one the default route leaves by), when it has one.
/// No packet is sent: connecting a UDP socket only picks the route.
pub fn lan_address() -> Option<IpAddr> {
    let socket = UdpSocket::bind(("0.0.0.0", 0)).ok()?;
    // TEST-NET-1 (RFC 5737): never a real host.
    socket.connect(("192.0.2.1", 9)).ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_loopback() && !ip.is_unspecified()).then_some(ip)
}

struct Session {
    server: solidcraft_mcp::Server,
    used: Instant,
}

fn serve(listener: TcpListener, token: &str, make: &MakeServer, ready: &Ready, stop: &AtomicBool, activity: &Mutex<Activity>) {
    let mut sessions: HashMap<String, Session> = HashMap::new();
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _)) => {
                if let Err(e) = connection(stream, token, make, ready, &mut sessions, activity) {
                    log::info!("mcp: connection: {e}");
                }
                activity.lock().unwrap_or_else(PoisonError::into_inner).sessions = sessions.len();
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(POLL),
            Err(e) => {
                log::warn!("mcp: accept: {e}");
                std::thread::sleep(POLL);
            }
        }
    }
}

/// A request as read off the connection.
#[derive(Debug, Default)]
struct Request {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }
}

/// A reply: status, extra headers, JSON body (empty for none).
struct Reply {
    status: u16,
    headers: Vec<(&'static str, String)>,
    body: String,
}

impl Reply {
    fn new(status: u16, body: impl Into<String>) -> Reply {
        Reply { status, headers: Vec::new(), body: body.into() }
    }

    fn error(status: u16, message: &str) -> Reply {
        Reply::new(status, serde_json::json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32000, "message": message}}).to_string())
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        202 => "Accepted",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        411 => "Length Required",
        413 => "Payload Too Large",
        _ => "Error",
    }
}

fn connection(
    stream: TcpStream,
    token: &str,
    make: &MakeServer,
    ready: &Ready,
    sessions: &mut HashMap<String, Session>,
    activity: &Mutex<Activity>,
) -> std::io::Result<()> {
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(READ_TIMEOUT))?;
    let mut writer = stream.try_clone()?;
    let mut reader = BufReader::new(stream);
    let reply = match read_request(&mut reader) {
        Ok(request) => handle(&request, token, make, ready, sessions, activity),
        Err(reply) => reply,
    };
    write_reply(&mut writer, &reply)
}

/// Read one HTTP/1.1 request, refusing what is too large or not understood.
fn read_request(reader: &mut impl BufRead) -> Result<Request, Reply> {
    let mut used = 0usize;
    let mut line = |reader: &mut dyn BufRead| -> Result<String, Reply> {
        let mut buf = Vec::new();
        let n = (&mut *reader)
            .take((MAX_HEADERS - used.min(MAX_HEADERS)) as u64 + 1)
            .read_until(b'\n', &mut buf)
            .map_err(|_| Reply::error(400, "bad request"))?;
        used += n;
        if used > MAX_HEADERS {
            return Err(Reply::error(413, "headers too large"));
        }
        String::from_utf8(buf).map(|s| s.trim_end_matches(['\r', '\n']).to_string()).map_err(|_| Reply::error(400, "headers are not UTF-8"))
    };
    let first = line(reader)?;
    let mut parts = first.split_whitespace();
    let (Some(method), Some(target)) = (parts.next(), parts.next()) else { return Err(Reply::error(400, "bad request line")) };
    let mut request = Request { method: method.to_ascii_uppercase(), path: target.split('?').next().unwrap_or("").to_string(), ..Default::default() };
    loop {
        let header = line(reader)?;
        if header.is_empty() {
            break;
        }
        if let Some((k, v)) = header.split_once(':') {
            request.headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    if request.header("transfer-encoding").is_some() {
        return Err(Reply::error(411, "send the body with a Content-Length"));
    }
    let length = match request.header("content-length") {
        None => 0,
        Some(v) => v.parse::<usize>().map_err(|_| Reply::error(400, "bad Content-Length"))?,
    };
    if length > MAX_BODY {
        return Err(Reply::error(413, "request too large"));
    }
    request.body = vec![0; length];
    reader.read_exact(&mut request.body).map_err(|_| Reply::error(400, "the body ended early"))?;
    Ok(request)
}

/// Compare without stopping at the first difference, so timing says nothing about the token.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn handle(
    request: &Request,
    token: &str,
    make: &MakeServer,
    ready: &Ready,
    sessions: &mut HashMap<String, Session>,
    activity: &Mutex<Activity>,
) -> Reply {
    if !matches!(request.path.trim_end_matches('/'), "/mcp" | "") {
        return Reply::error(404, "the MCP endpoint is /mcp");
    }
    let bearer = request.header("authorization").and_then(|v| v.strip_prefix("Bearer ").or_else(|| v.strip_prefix("bearer ")));
    if token.is_empty() || !bearer.is_some_and(|b| same(b.trim(), token)) {
        activity.lock().unwrap_or_else(PoisonError::into_inner).refused += 1;
        let mut reply = Reply::error(401, "a valid access token is needed: copy the command from SolidCraft's Settings › MCP server again");
        reply.headers.push(("WWW-Authenticate", "Bearer".into()));
        return reply;
    }
    {
        let mut a = activity.lock().unwrap_or_else(PoisonError::into_inner);
        a.requests += 1;
        a.last_request = Some(Instant::now());
    }
    let session_id = request.header("mcp-session-id").map(str::to_string);
    match request.method.as_str() {
        "POST" => post(request, session_id, make, ready, sessions),
        "DELETE" => match session_id.and_then(|id| sessions.remove(&id)) {
            Some(_) => Reply::new(200, ""),
            None => Reply::error(404, "no such session"),
        },
        // No event stream: everything comes back in the POST's own reply.
        _ => {
            let mut reply = Reply::new(405, "");
            reply.headers.push(("Allow", "POST, DELETE".into()));
            reply
        }
    }
}

/// The methods that reach the app (the others the MCP server answers itself).
fn reaches_app(message: &serde_json::Value) -> bool {
    match message {
        serde_json::Value::Array(batch) => batch.iter().any(reaches_app),
        m => matches!(m.get("method").and_then(|m| m.as_str()), Some("tools/call" | "resources/read")),
    }
}

/// Wait a moment for the app to come back on screen; `false`: it didn't.
fn wait_ready(ready: &Ready) -> bool {
    let deadline = Instant::now() + READY_WAIT;
    while !ready() {
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    true
}

/// The answer to a call the app couldn't take: a tool error the agent reads, else a JSON-RPC one.
fn not_on_screen(message: &serde_json::Value) -> String {
    let id = message.get("id").cloned().unwrap_or(serde_json::Value::Null);
    if message.get("method").and_then(|m| m.as_str()) == Some("tools/call") {
        serde_json::json!({"jsonrpc": "2.0", "id": id, "result": {"content": [{"type": "text", "text": NOT_ON_SCREEN}], "isError": true}})
    } else {
        serde_json::json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32000, "message": NOT_ON_SCREEN}})
    }
    .to_string()
}

fn post(request: &Request, session_id: Option<String>, make: &MakeServer, ready: &Ready, sessions: &mut HashMap<String, Session>) -> Reply {
    let Ok(body) = std::str::from_utf8(&request.body) else { return Reply::error(400, "the body is not UTF-8") };
    let message = serde_json::from_str::<serde_json::Value>(body).unwrap_or_default();
    let initialize = message.get("method").and_then(|m| m.as_str()) == Some("initialize");
    let id = if initialize {
        // A new session, even when the client sends an old id with it.
        let Some(id) = new_token().map(|t| t.chars().take(32).collect::<String>()) else { return Reply::error(500, "no random session id") };
        if sessions.len() >= MAX_SESSIONS
            && let Some(oldest) = sessions.iter().min_by_key(|(_, s)| s.used).map(|(k, _)| k.clone())
        {
            sessions.remove(&oldest);
        }
        sessions.insert(id.clone(), Session { server: make(), used: Instant::now() });
        id
    } else {
        match session_id {
            Some(id) if sessions.contains_key(&id) => id,
            // The client re-initializes on 404 (the app was restarted, or the session evicted).
            Some(_) => return Reply::error(404, "session not found: start a new one"),
            // A client that doesn't keep sessions gets one of its own, kept as for any other.
            None => {
                let id = String::from("-");
                sessions.entry(id.clone()).or_insert_with(|| Session { server: make(), used: Instant::now() });
                id
            }
        }
    };
    let Some(session) = sessions.get_mut(&id) else { return Reply::error(500, "session lost") };
    session.used = Instant::now();
    let answer = if reaches_app(&message) && !wait_ready(ready) { Some(not_on_screen(&message)) } else { session.server.handle_line(body) };
    let mut reply = match answer {
        Some(json) => Reply::new(200, json),
        None => Reply::new(202, ""),
    };
    if id != "-" {
        reply.headers.push(("Mcp-Session-Id", id));
    }
    reply
}

fn write_reply(w: &mut impl Write, reply: &Reply) -> std::io::Result<()> {
    let mut head = format!("HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n", reply.status, reason(reply.status), reply.body.len());
    if !reply.body.is_empty() {
        head.push_str("Content-Type: application/json\r\n");
    }
    for (k, v) in &reply.headers {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str("\r\n");
    w.write_all(head.as_bytes())?;
    w.write_all(reply.body.as_bytes())?;
    w.flush()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn endpoint() -> Endpoint {
        let make: MakeServer = Box::new(|| solidcraft_mcp::Server::new(Box::new(solidcraft_mcp::Headless::default())));
        Endpoint::start("127.0.0.1:0".parse().unwrap(), "s3cret".into(), make, Box::new(|| true)).unwrap()
    }

    /// One request; the status, the headers (lower-case names) and the body.
    fn send(port: u16, method: &str, headers: &[(&str, &str)], body: &str) -> (u16, Vec<(String, String)>, String) {
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let mut req = format!("{method} /mcp HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n", body.len());
        for (k, v) in headers {
            req.push_str(&format!("{k}: {v}\r\n"));
        }
        req.push_str("\r\n");
        req.push_str(body);
        s.write_all(req.as_bytes()).unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        let (head, body) = out.split_once("\r\n\r\n").unwrap();
        let mut lines = head.lines();
        let status = lines.next().unwrap().split_whitespace().nth(1).unwrap().parse().unwrap();
        let headers = lines.filter_map(|l| l.split_once(':')).map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string())).collect();
        (status, headers, body.to_string())
    }

    fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
        headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    const AUTH: (&str, &str) = ("Authorization", "Bearer s3cret");

    #[test]
    fn a_client_initializes_lists_tools_and_models_in_its_session() {
        let ep = endpoint();
        let init = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "t", "version": "1"}}});
        let (status, headers, body) = send(ep.port(), "POST", &[AUTH], &init.to_string());
        assert_eq!(status, 200, "{body}");
        let session = header(&headers, "mcp-session-id").unwrap().to_string();
        assert_eq!(serde_json::from_str::<Value>(&body).unwrap()["result"]["serverInfo"]["name"], "solidcraft");
        let s = ("Mcp-Session-Id", session.as_str());
        let (status, _, body) = send(ep.port(), "POST", &[AUTH, s], &json!({"jsonrpc": "2.0", "method": "notifications/initialized"}).to_string());
        assert_eq!((status, body.as_str()), (202, ""));
        let (status, _, body) = send(ep.port(), "POST", &[AUTH, s], &json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}).to_string());
        assert_eq!(status, 200);
        let tools: Value = serde_json::from_str(&body).unwrap();
        assert!(tools["result"]["tools"].as_array().unwrap().iter().any(|t| t["name"] == "execute"));
        let call = json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "execute", "arguments": {"command": "solid.box", "params": {"length": 10, "width": 10, "height": 10}}}});
        let (status, _, body) = send(ep.port(), "POST", &[AUTH, s], &call.to_string());
        assert_eq!(status, 200);
        assert_ne!(serde_json::from_str::<Value>(&body).unwrap()["result"]["isError"], json!(true), "{body}");
        assert_eq!(ep.activity().sessions, 1);
        let (status, _, _) = send(ep.port(), "DELETE", &[AUTH, s], "");
        assert_eq!(status, 200);
        let (status, _, _) = send(ep.port(), "POST", &[AUTH, s], &json!({"jsonrpc": "2.0", "id": 4, "method": "tools/list"}).to_string());
        assert_eq!(status, 404, "a closed session is gone");
    }

    #[test]
    fn without_the_token_nothing_runs() {
        let ep = endpoint();
        let list = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}).to_string();
        for headers in [&[][..], &[("Authorization", "Bearer wrong!")][..], &[("Authorization", "s3cret")][..]] {
            let (status, h, _) = send(ep.port(), "POST", headers, &list);
            assert_eq!(status, 401);
            assert_eq!(header(&h, "www-authenticate"), Some("Bearer"));
        }
        assert_eq!(ep.activity().refused, 3);
        assert_eq!(ep.activity().requests, 0);
    }

    #[test]
    fn no_event_stream_and_hostile_requests_are_refused() {
        let ep = endpoint();
        assert_eq!(send(ep.port(), "GET", &[AUTH], "").0, 405);
        let mut s = TcpStream::connect(("127.0.0.1", ep.port())).unwrap();
        s.write_all(format!("POST /mcp HTTP/1.1\r\nAuthorization: Bearer s3cret\r\nContent-Length: {}\r\n\r\n", MAX_BODY + 1).as_bytes()).unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        assert!(out.starts_with("HTTP/1.1 413"), "{out}");
        let mut s = TcpStream::connect(("127.0.0.1", ep.port())).unwrap();
        s.write_all(format!("POST /mcp HTTP/1.1\r\nX: {}\r\n", "a".repeat(MAX_HEADERS)).as_bytes()).unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        assert!(out.starts_with("HTTP/1.1 413"), "{out}");
        let (status, _, body) = send(ep.port(), "POST", &[AUTH], "{not json");
        assert_eq!(status, 200);
        assert_eq!(serde_json::from_str::<Value>(&body).unwrap()["error"]["code"], -32700);
    }

    #[test]
    fn a_stopped_endpoint_frees_its_port() {
        let mut ep = endpoint();
        let port = ep.port();
        ep.stop();
        let again = Endpoint::start(
            SocketAddr::from(([127, 0, 0, 1], port)),
            "t".into(),
            Box::new(|| solidcraft_mcp::Server::new(Box::new(solidcraft_mcp::Headless::default()))),
            Box::new(|| true),
        );
        assert!(again.is_ok());
    }

    #[test]
    fn a_tool_call_while_the_app_is_off_screen_says_so_instead_of_hanging() {
        let make: MakeServer = Box::new(|| solidcraft_mcp::Server::new(Box::new(solidcraft_mcp::Headless::default())));
        let shown = Arc::new(AtomicBool::new(false));
        let s = shown.clone();
        let ep = Endpoint::start("127.0.0.1:0".parse().unwrap(), "s3cret".into(), make, Box::new(move || s.load(Ordering::Relaxed))).unwrap();
        // Listing tools needs no app.
        let (status, _, body) = send(ep.port(), "POST", &[AUTH], &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}).to_string());
        assert_eq!(status, 200);
        assert!(body.contains("\"tools\""));
        // Back on screen within the wait: the call runs.
        let back = shown.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(400));
            back.store(true, Ordering::Relaxed);
        });
        let call = json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "list_commands", "arguments": {}}}).to_string();
        let (_, _, body) = send(ep.port(), "POST", &[AUTH], &call);
        assert_ne!(serde_json::from_str::<Value>(&body).unwrap()["result"]["isError"], json!(true), "{body}");
        assert!(!body.contains("back on screen"));
        let call = json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "list_commands", "arguments": {}}});
        let answer: Value = serde_json::from_str(&not_on_screen(&call)).unwrap();
        assert_eq!(answer["id"], 3);
        assert_eq!(answer["result"]["isError"], true);
        assert!(answer["result"]["content"][0]["text"].as_str().unwrap().contains("back on screen"));
    }

    #[test]
    fn tokens_are_long_and_fresh() {
        let (a, b) = (new_token().unwrap(), new_token().unwrap());
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
        assert!(same("abc", "abc") && !same("abc", "abd") && !same("abc", "abcd"));
    }
}
