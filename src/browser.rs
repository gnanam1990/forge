//! Browser automation via the Chrome DevTools Protocol (CDP). Launches a
//! headless Chromium with remote debugging and drives it over the DevTools HTTP
//! endpoint (open pages, list targets). Full WebSocket evaluation is a future
//! enhancement.

use std::process::{Child, Command, Stdio};

use serde_json::Value;

use crate::error::{Error, Result};

/// A headless browser session.
pub struct Browser {
    port: u16,
    child: Option<Child>,
    client: reqwest::blocking::Client,
}

impl Browser {
    /// Launch a headless Chromium/Chrome with remote debugging on a free port.
    pub fn launch() -> Result<Self> {
        let port = pick_free_port()?;
        let child = Command::new("google-chrome")
            .arg("--headless=new")
            .arg("--disable-gpu")
            .arg("--no-sandbox")
            .arg(format!("--remote-debugging-port={port}"))
            .arg("about:blank")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .or_else(|_| {
                Command::new("chromium")
                    .arg("--headless=new")
                    .arg("--disable-gpu")
                    .arg("--no-sandbox")
                    .arg(format!("--remote-debugging-port={port}"))
                    .arg("about:blank")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
            })
            .or_else(|_| {
                Command::new("chromium-browser")
                    .arg("--headless=new")
                    .arg("--disable-gpu")
                    .arg("--no-sandbox")
                    .arg(format!("--remote-debugging-port={port}"))
                    .arg("about:blank")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
            })
            .map_err(|e| Error::Agent(format!("launch browser: {e}")))?;
        Ok(Browser {
            port,
            child: Some(child),
            client: reqwest::blocking::Client::new(),
        })
    }

    /// Open a URL in a new tab and return the target id.
    pub fn open(&self, url: &str) -> Result<String> {
        let endpoint = format!("http://127.0.0.1:{}/json/new?{}", self.port, urlencode(url));
        let response = self
            .client
            .put(&endpoint)
            .send()
            .map_err(|e| Error::Agent(format!("open page: {e}")))?;
        let value: Value = response
            .json()
            .map_err(|e| Error::Agent(format!("parse target: {e}")))?;
        value
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| Error::Agent("no target id in response".into()))
    }

    /// List the open targets (tabs).
    pub fn list(&self) -> Result<Vec<String>> {
        let endpoint = format!("http://127.0.0.1:{}/json/list", self.port);
        let response = self
            .client
            .get(&endpoint)
            .send()
            .map_err(|e| Error::Agent(format!("list targets: {e}")))?;
        let value: Value = response
            .json()
            .map_err(|e| Error::Agent(format!("parse targets: {e}")))?;
        Ok(value
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| t.get("url").and_then(Value::as_str).map(str::to_string))
                    .collect()
            })
            .unwrap_or_default())
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Pick a free TCP port by binding a listener and dropping it.
fn pick_free_port() -> Result<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|e| Error::Agent(format!("bind port: {e}")))?;
    Ok(listener
        .local_addr()
        .map_err(|e| Error::Agent(e.to_string()))?
        .port())
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urlencodes() {
        assert_eq!(
            urlencode("https://example.com/a b"),
            "https%3A%2F%2Fexample.com%2Fa%20b"
        );
    }
}
