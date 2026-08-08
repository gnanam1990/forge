//! A minimal, dependency-free HTTP API server for forge.
//!
//! `forge serve` exposes the agent over HTTP so other tools and scripts can
//! drive it. The server is intentionally small: it uses only `std::net` and
//! speaks a tiny subset of HTTP/1.1 (request line, headers, `Content-Length`
//! body). Endpoints:
//!
//! - `GET  /health`   — liveness probe
//! - `POST /run`      — run the agent on a prompt (`{"prompt": "..."}`)
//! - `GET  /sessions` — list saved session ids
//! - `GET  /memory`   — list remembered facts

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

use serde_json::json;

use crate::config::Config;
use crate::error::{Error, Result};

/// A parsed HTTP request.
struct Request {
    method: String,
    path: String,
    body: String,
}

/// Run the HTTP server on the given bind address until interrupted.
pub fn serve(bind: &str) -> Result<()> {
    let listener =
        TcpListener::bind(bind).map_err(|e| Error::Config(format!("bind {bind}: {e}")))?;
    eprintln!("[forge] serving on http://{bind}");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(e) = handle_connection(stream) {
                    eprintln!("[forge] request error: {e}");
                }
            }
            Err(e) => eprintln!("[forge] accept error: {e}"),
        }
    }
    Ok(())
}

/// Handle a single connection: read the request, route it, and write a reply.
fn handle_connection(stream: TcpStream) -> Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line)? == 0 {
        return Ok(());
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("/").to_string();

    // Read headers to find Content-Length.
    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some((name, value)) = trimmed.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().unwrap_or(0);
            }
        }
    }

    let mut body = String::new();
    if content_length > 0 {
        let mut buf = vec![0u8; content_length];
        reader.read_exact(&mut buf)?;
        body = String::from_utf8_lossy(&buf).into_owned();
    }

    let request = Request { method, path, body };
    match route(&request) {
        Ok((status, content_type, payload)) => {
            write_response(stream, status, content_type, &payload)
        }
        Err(e) => {
            let payload = json!({ "error": e.to_string() }).to_string();
            write_response(stream, 500, "application/json", &payload)
        }
    }
}

/// Route a request to the appropriate handler.
fn route(req: &Request) -> Result<(u16, &'static str, String)> {
    let config = Config::load()?;
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/health") => Ok((
            200,
            "application/json",
            json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }).to_string(),
        )),
        ("POST", "/run") => {
            let prompt: serde_json::Value = serde_json::from_str(&req.body)
                .map_err(|e| Error::InvalidArgs(format!("bad JSON body: {e}")))?;
            let prompt = prompt
                .get("prompt")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| Error::InvalidArgs("body must be {\"prompt\": string}".into()))?;
            let outcome = run_agent(&config, prompt)?;
            Ok((
                200,
                "application/json",
                json!({
                    "output": outcome.final_text,
                    "turns": outcome.turns,
                    "tool_calls": outcome.tool_calls,
                })
                .to_string(),
            ))
        }
        ("GET", "/sessions") => {
            let dir = crate::session::default_sessions_dir()?;
            let sessions = crate::session::Session::list(&dir)?;
            Ok((
                200,
                "application/json",
                json!({ "sessions": sessions }).to_string(),
            ))
        }
        ("GET", "/memory") => {
            let path = crate::memory::default_memory_path()?;
            let memory = crate::memory::Memory::load(path)?;
            let facts: serde_json::Map<String, serde_json::Value> = memory
                .all()
                .into_iter()
                .map(|(k, v)| (k, serde_json::Value::String(v)))
                .collect();
            Ok((
                200,
                "application/json",
                json!({ "memory": facts }).to_string(),
            ))
        }
        _ => Ok((
            404,
            "application/json",
            json!({ "error": "not found" }).to_string(),
        )),
    }
}

/// Run the agent once on a prompt and return the outcome.
fn run_agent(config: &Config, prompt: &str) -> Result<crate::agent::AgentOutcome> {
    let turns = config.max_turns.unwrap_or(10);
    let provider = crate::agent::http::HttpProvider::new(&config.provider)?;
    let wiring = crate::wiring::build_wiring(config)?;
    let agent = crate::agent::Agent::new(Box::new(provider), wiring.registry, turns)
        .with_hooks(wiring.hooks);
    let dir = crate::session::default_sessions_dir()?;
    let id = format!(
        "sess-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    );
    let mut session = crate::session::Session::new(&id);
    let outcome = agent.run_into(&mut session.messages, prompt)?;
    session.save(&dir)?;
    Ok(outcome)
}

/// Write an HTTP/1.1 response to the stream.
fn write_response(
    mut stream: TcpStream,
    status: u16,
    content_type: &str,
    body: &str,
) -> Result<()> {
    let reason = match status {
        200 => "OK",
        404 => "Not Found",
        400 => "Bad Request",
        500 => "Internal Server Error",
        _ => "OK",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    stream.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_health() {
        let req = Request {
            method: "GET".into(),
            path: "/health".into(),
            body: String::new(),
        };
        let (status, ct, body) = route(&req).unwrap();
        assert_eq!(status, 200);
        assert_eq!(ct, "application/json");
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["status"], "ok");
    }

    #[test]
    fn routes_unknown_to_404() {
        let req = Request {
            method: "GET".into(),
            path: "/nope".into(),
            body: String::new(),
        };
        let (status, _, _) = route(&req).unwrap();
        assert_eq!(status, 404);
    }

    #[test]
    fn run_rejects_missing_prompt() {
        let req = Request {
            method: "POST".into(),
            path: "/run".into(),
            body: "{}".into(),
        };
        assert!(route(&req).is_err());
    }
}
