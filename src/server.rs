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
        ("GET", "/version") => Ok((
            200,
            "application/json",
            json!({ "version": env!("CARGO_PKG_VERSION") }).to_string(),
        )),
        ("POST", "/tools/call") => {
            let body: serde_json::Value = serde_json::from_str(&req.body)
                .map_err(|e| Error::InvalidArgs(format!("bad JSON body: {e}")))?;
            let tool_name = body
                .get("tool")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| Error::InvalidArgs("body must be {\"tool\": string, ...}".into()))?;
            let args = body.get("args").cloned().unwrap_or(serde_json::Value::Null);
            let wiring = crate::wiring::build_wiring(&config)?;
            let tool = wiring
                .registry
                .get(tool_name)
                .ok_or_else(|| Error::InvalidArgs(format!("no tool named {tool_name}")))?;
            let ctx = crate::tools::ToolContext::new(config.workspace_root());
            let result = crate::tools::Tool::run(tool, &args, &ctx)?;
            Ok((
                200,
                "application/json",
                json!({
                    "ok": result.ok,
                    "output": result.output,
                    "error": result.error,
                })
                .to_string(),
            ))
        }
        ("GET", "/tools") => {
            let names = crate::tools::Registry::builtin().names();
            Ok((
                200,
                "application/json",
                json!({ "tools": names }).to_string(),
            ))
        }
        ("GET", "/config") => Ok((
            200,
            "application/json",
            json!({
                "workspace": config.workspace_root().display().to_string(),
                "model": config.provider.model.as_deref().unwrap_or("(none)"),
                "max_turns": config.max_turns.unwrap_or(10),
                "telemetry": config.telemetry,
            })
            .to_string(),
        )),
        ("POST", "/run") => {
            let body: serde_json::Value = serde_json::from_str(&req.body)
                .map_err(|e| Error::InvalidArgs(format!("bad JSON body: {e}")))?;
            let prompt = body
                .get("prompt")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| Error::InvalidArgs("body must be {\"prompt\": string}".into()))?;
            let resume = body
                .get("session")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);
            let max_turns = body
                .get("max_turns")
                .and_then(serde_json::Value::as_u64)
                .map(|t| t as usize);
            let outcome = run_agent(&config, prompt, resume.as_deref(), max_turns)?;
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
        ("POST", "/memory") => {
            let value: serde_json::Value = serde_json::from_str(&req.body)
                .map_err(|e| Error::InvalidArgs(format!("bad JSON body: {e}")))?;
            let key = value
                .get("key")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| Error::InvalidArgs("body must be {\"key\": string, ...}".into()))?;
            let val = value
                .get("value")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    Error::InvalidArgs("body must be {\"key\": string, \"value\": string}".into())
                })?;
            let path = crate::memory::default_memory_path()?;
            let mut memory = crate::memory::Memory::load(path)?;
            memory.remember(key, val);
            memory.save()?;
            Ok((
                200,
                "application/json",
                json!({ "ok": true, "key": key }).to_string(),
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
        ("DELETE", _) => {
            // `DELETE /memory/<key>`
            let rest = req.path.trim_start_matches("/memory/");
            if req.path.starts_with("/memory/") && !rest.is_empty() {
                let path = crate::memory::default_memory_path()?;
                let mut memory = crate::memory::Memory::load(path)?;
                let removed = memory.forget(rest);
                if removed {
                    memory.save()?;
                    Ok((
                        200,
                        "application/json",
                        json!({ "ok": true, "key": rest }).to_string(),
                    ))
                } else {
                    Ok((
                        404,
                        "application/json",
                        json!({ "error": format!("no fact named {rest}") }).to_string(),
                    ))
                }
            } else {
                Ok((
                    404,
                    "application/json",
                    json!({ "error": "not found" }).to_string(),
                ))
            }
        }
        ("GET", "/stats") => {
            let path = crate::telemetry::default_telemetry_path()?;
            let raw = std::fs::read_to_string(&path).unwrap_or_default();
            let mut counts: std::collections::BTreeMap<String, usize> =
                std::collections::BTreeMap::new();
            let mut total = 0usize;
            for line in raw.lines() {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                    if let Some(event) = value.get("event").and_then(|e| e.as_str()) {
                        *counts.entry(event.to_string()).or_insert(0) += 1;
                        total += 1;
                    }
                }
            }
            Ok((
                200,
                "application/json",
                json!({ "total": total, "events": counts }).to_string(),
            ))
        }
        ("GET", "/plugins") => {
            let dir = config
                .plugins_dir
                .clone()
                .unwrap_or_else(|| config.workspace_root().join(".forge").join("plugins"));
            let state_path = dir.join("state.json");
            let mut registry = crate::plugin::PluginRegistry::new(state_path);
            registry.load_dir(&dir)?;
            let plugins: Vec<serde_json::Value> = registry
                .list()
                .iter()
                .map(|p| {
                    json!({
                        "name": p.name,
                        "enabled": p.enabled,
                        "tools": p.tools,
                    })
                })
                .collect();
            Ok((
                200,
                "application/json",
                json!({ "plugins": plugins }).to_string(),
            ))
        }
        ("POST", "/memory/forget") => {
            let value: serde_json::Value = serde_json::from_str(&req.body)
                .map_err(|e| Error::InvalidArgs(format!("bad JSON body: {e}")))?;
            let key = value
                .get("key")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| Error::InvalidArgs("body must be {\"key\": string}".into()))?;
            let path = crate::memory::default_memory_path()?;
            let mut memory = crate::memory::Memory::load(path)?;
            let removed = memory.forget(key);
            if removed {
                memory.save()?;
                Ok((
                    200,
                    "application/json",
                    json!({ "ok": true, "key": key }).to_string(),
                ))
            } else {
                Ok((
                    404,
                    "application/json",
                    json!({ "error": format!("no fact named {key}") }).to_string(),
                ))
            }
        }
        ("GET", _) => {
            // `GET /session/<id>`
            if req.path.starts_with("/session/") {
                let id = req.path.trim_start_matches("/session/");
                if id.is_empty() {
                    return Ok((
                        404,
                        "application/json",
                        json!({ "error": "missing session id" }).to_string(),
                    ));
                }
                let dir = crate::session::default_sessions_dir()?;
                match crate::session::Session::load(&dir, id) {
                    Ok(session) => Ok((
                        200,
                        "application/json",
                        json!({
                            "id": session.id,
                            "messages": session.message_count(),
                            "tokens": session.token_usage(),
                            "created_at": session.created_at,
                        })
                        .to_string(),
                    )),
                    Err(_) => Ok((
                        404,
                        "application/json",
                        json!({ "error": format!("no session {id}") }).to_string(),
                    )),
                }
            } else {
                Ok((
                    404,
                    "application/json",
                    json!({ "error": "not found" }).to_string(),
                ))
            }
        }
        _ => Ok((
            404,
            "application/json",
            json!({ "error": "not found" }).to_string(),
        )),
    }
}

/// Run the agent once on a prompt and return the outcome.
fn run_agent(
    config: &Config,
    prompt: &str,
    resume: Option<&str>,
    max_turns: Option<usize>,
) -> Result<crate::agent::AgentOutcome> {
    let turns = max_turns.or(config.max_turns).unwrap_or(10);
    let provider = crate::agent::http::HttpProvider::new(&config.provider)?;
    let wiring = crate::wiring::build_wiring(config)?;
    let agent = crate::agent::Agent::new(Box::new(provider), wiring.registry, turns)
        .with_hooks(wiring.hooks);
    let dir = crate::session::default_sessions_dir()?;
    let mut session = if let Some(id) = resume {
        crate::session::Session::load(&dir, id)?
    } else {
        let id = format!(
            "sess-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        );
        crate::session::Session::new(&id)
    };
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

    /// Serializes tests that mutate process-global env vars (FORGE_*), since
    /// Rust runs tests in parallel threads sharing the environment.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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

    #[test]
    fn routes_tools() {
        let req = Request {
            method: "GET".into(),
            path: "/tools".into(),
            body: String::new(),
        };
        let (status, ct, body) = route(&req).unwrap();
        assert_eq!(status, 200);
        assert_eq!(ct, "application/json");
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        let tools = v["tools"].as_array().unwrap();
        assert!(tools.iter().any(|t| t == "read_file"));
        assert!(tools.iter().any(|t| t == "move_file"));
    }

    #[test]
    fn routes_config() {
        let req = Request {
            method: "GET".into(),
            path: "/config".into(),
            body: String::new(),
        };
        let (status, _, body) = route(&req).unwrap();
        assert_eq!(status, 200);
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert!(v.get("workspace").is_some());
        assert!(v.get("model").is_some());
    }

    #[test]
    fn memory_post_and_delete_roundtrip() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("FORGE_MEMORY", dir.path().join("memory.json"));
        let post = Request {
            method: "POST".into(),
            path: "/memory".into(),
            body: r#"{"key":"lang","value":"rust"}"#.into(),
        };
        let (status, _, _) = route(&post).unwrap();
        assert_eq!(status, 200);

        let get = Request {
            method: "GET".into(),
            path: "/memory".into(),
            body: String::new(),
        };
        let (_, _, body) = route(&get).unwrap();
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["memory"]["lang"], "rust");

        let del = Request {
            method: "DELETE".into(),
            path: "/memory/lang".into(),
            body: String::new(),
        };
        let (status, _, _) = route(&del).unwrap();
        assert_eq!(status, 200);

        let get2 = Request {
            method: "GET".into(),
            path: "/memory".into(),
            body: String::new(),
        };
        let (_, _, body) = route(&get2).unwrap();
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert!(v["memory"].as_object().unwrap().is_empty());

        std::env::remove_var("FORGE_MEMORY");
    }

    #[test]
    fn delete_missing_memory_is_404() {
        let req = Request {
            method: "DELETE".into(),
            path: "/memory/does_not_exist".into(),
            body: String::new(),
        };
        let (status, _, _) = route(&req).unwrap();
        assert_eq!(status, 404);
    }

    #[test]
    fn routes_stats() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("FORGE_TELEMETRY", dir.path().join("t.jsonl"));
        std::fs::write(
            dir.path().join("t.jsonl"),
            "{\"event\":\"run\",\"data\":{}}\n{\"event\":\"run\",\"data\":{}}\n",
        )
        .unwrap();
        let req = Request {
            method: "GET".into(),
            path: "/stats".into(),
            body: String::new(),
        };
        let (status, _, body) = route(&req).unwrap();
        assert_eq!(status, 200);
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["total"], 2);
        assert_eq!(v["events"]["run"], 2);
        std::env::remove_var("FORGE_TELEMETRY");
    }

    #[test]
    fn routes_session_by_id() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let session_dir = dir.path().join("sessions");
        let session = crate::session::Session::new("sess-1");
        session.save(&session_dir).unwrap();
        std::env::set_var("FORGE_SESSIONS_DIR", session_dir.display().to_string());
        let req = Request {
            method: "GET".into(),
            path: "/session/sess-1".into(),
            body: String::new(),
        };
        let (status, _, body) = route(&req).unwrap();
        assert_eq!(status, 200);
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["id"], "sess-1");
        std::env::remove_var("FORGE_SESSIONS_DIR");
    }

    #[test]
    fn session_missing_is_404() {
        let req = Request {
            method: "GET".into(),
            path: "/session/does_not_exist".into(),
            body: String::new(),
        };
        let (status, _, _) = route(&req).unwrap();
        assert_eq!(status, 404);
    }

    #[test]
    fn routes_version() {
        let req = Request {
            method: "GET".into(),
            path: "/version".into(),
            body: String::new(),
        };
        let (status, _, body) = route(&req).unwrap();
        assert_eq!(status, 200);
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert!(v.get("version").is_some());
    }

    #[test]
    fn tools_call_reads_a_file() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "hello").unwrap();
        let config_path = dir.path().join("config.json");
        std::fs::write(
            &config_path,
            format!(r#"{{"workspace":"{}"}}"#, dir.path().display()),
        )
        .unwrap();
        std::env::set_var("FORGE_CONFIG", config_path.display().to_string());
        let req = Request {
            method: "POST".into(),
            path: "/tools/call".into(),
            body: r#"{"tool":"read_file","args":{"path":"a.txt"}}"#.into(),
        };
        let (status, _, body) = route(&req).unwrap();
        assert_eq!(status, 200);
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["ok"], true);
        assert!(v["output"].as_str().unwrap().contains("hello"));
        std::env::remove_var("FORGE_CONFIG");
    }

    #[test]
    fn tools_call_unknown_tool_is_error() {
        let req = Request {
            method: "POST".into(),
            path: "/tools/call".into(),
            body: r#"{"tool":"does_not_exist","args":{}}"#.into(),
        };
        assert!(route(&req).is_err());
    }

    #[test]
    fn routes_plugins() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let plugins_dir = dir.path().join(".forge/plugins");
        std::fs::create_dir_all(&plugins_dir).unwrap();
        std::fs::write(
            plugins_dir.join("demo.json"),
            r#"{"name":"demo","tools":[{"name":"greet","command":"echo","description":"greets"}]}"#,
        )
        .unwrap();
        let config_path = dir.path().join("config.json");
        std::fs::write(
            &config_path,
            format!(
                r#"{{"workspace":"{}","plugins_dir":"{}"}}"#,
                dir.path().display(),
                plugins_dir.display()
            ),
        )
        .unwrap();
        std::env::set_var("FORGE_CONFIG", config_path.display().to_string());
        let req = Request {
            method: "GET".into(),
            path: "/plugins".into(),
            body: String::new(),
        };
        let (status, _, body) = route(&req).unwrap();
        assert_eq!(status, 200);
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["plugins"][0]["name"], "demo");
        std::env::remove_var("FORGE_CONFIG");
    }

    #[test]
    fn memory_forget_roundtrip() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("FORGE_MEMORY", dir.path().join("memory.json"));
        let post = Request {
            method: "POST".into(),
            path: "/memory".into(),
            body: r#"{"key":"lang","value":"rust"}"#.into(),
        };
        route(&post).unwrap();
        let forget = Request {
            method: "POST".into(),
            path: "/memory/forget".into(),
            body: r#"{"key":"lang"}"#.into(),
        };
        let (status, _, _) = route(&forget).unwrap();
        assert_eq!(status, 200);
        let get = Request {
            method: "GET".into(),
            path: "/memory".into(),
            body: String::new(),
        };
        let (_, _, body) = route(&get).unwrap();
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert!(v["memory"].as_object().unwrap().is_empty());
        std::env::remove_var("FORGE_MEMORY");
    }
}
