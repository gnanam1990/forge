//! Browser automation via the Chrome DevTools Protocol (CDP). Launches a
//! headless Chromium with remote debugging and drives it over the DevTools HTTP
//! endpoint (open pages, list targets). Full WebSocket evaluation is a future
//! enhancement.

use std::process::{Child, Command, Stdio};

use serde_json::{json, Value};

use crate::error::{Error, Result};

/// A browser target (tab) with its DevTools WebSocket URL.
#[derive(Debug, Clone)]
pub struct Target {
    pub id: String,
    pub ws_url: String,
}

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

    /// Open a URL in a new tab and return the target.
    pub fn open(&self, url: &str) -> Result<Target> {
        let endpoint = format!("http://127.0.0.1:{}/json/new?{}", self.port, urlencode(url));
        let response = self
            .client
            .put(&endpoint)
            .send()
            .map_err(|e| Error::Agent(format!("open page: {e}")))?;
        let value: Value = response
            .json()
            .map_err(|e| Error::Agent(format!("parse target: {e}")))?;
        let id = value
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| Error::Agent("no target id in response".into()))?;
        let ws_url = value
            .get("webSocketDebuggerUrl")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| Error::Agent("no websocket url in response".into()))?;
        Ok(Target { id, ws_url })
    }

    /// Evaluate a JavaScript expression in a target over the DevTools
    /// WebSocket and return the result value.
    pub fn evaluate(&self, target: &Target, expression: &str) -> Result<String> {
        let result = self.send_command(
            target,
            "Runtime.evaluate",
            json!({
                "expression": expression,
                "returnByValue": true
            }),
        )?;
        result
            .get("result")
            .and_then(|r| r.get("value"))
            .map(|v| v.to_string())
            .ok_or_else(|| Error::Agent("no value in evaluate result".into()))
    }

    /// Navigate the target to a URL.
    pub fn navigate(&self, target: &Target, url: &str) -> Result<()> {
        self.send_command(target, "Page.navigate", json!({ "url": url }))?;
        Ok(())
    }

    /// Go back in the target's history.
    pub fn back(&self, target: &Target) -> Result<()> {
        self.send_command(target, "Page.goBack", json!({}))?;
        Ok(())
    }

    /// Go forward in the target's history.
    pub fn forward(&self, target: &Target) -> Result<()> {
        self.send_command(target, "Page.goForward", json!({}))?;
        Ok(())
    }

    /// Reload the target.
    pub fn reload(&self, target: &Target) -> Result<()> {
        self.send_command(target, "Page.reload", json!({}))?;
        Ok(())
    }

    /// Get the visible text of the target's page.
    pub fn get_text(&self, target: &Target) -> Result<String> {
        self.evaluate(target, "document.body ? document.body.innerText : ''")
    }

    /// Get the title of the target's page.
    pub fn get_title(&self, target: &Target) -> Result<String> {
        self.evaluate(target, "document.title")
    }

    /// Get the current URL of the target's page.
    pub fn get_url(&self, target: &Target) -> Result<String> {
        self.evaluate(target, "location.href")
    }

    /// Get the full HTML of the target's page.
    pub fn get_html(&self, target: &Target) -> Result<String> {
        self.evaluate(target, "document.documentElement.outerHTML")
    }

    /// Wait until the page finishes loading, up to a timeout.
    pub fn wait_for_load(&self, target: &Target, timeout_secs: u64) -> Result<()> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
        loop {
            let state = self
                .evaluate(target, "document.readyState")
                .unwrap_or_default();
            if state.trim_matches('"') == "complete" {
                return Ok(());
            }
            if std::time::Instant::now() >= deadline {
                return Err(Error::Agent("page did not finish loading in time".into()));
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }

    /// Wait until a CSS selector matches an element, up to a timeout.
    pub fn wait_for_selector(
        &self,
        target: &Target,
        selector: &str,
        timeout_secs: u64,
    ) -> Result<()> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
        let expr = format!(
            "!!document.querySelector({})",
            serde_json::to_string(selector)?
        );
        loop {
            let found = self.evaluate(target, &expr).unwrap_or_default();
            if found == "true" {
                return Ok(());
            }
            if std::time::Instant::now() >= deadline {
                return Err(Error::Agent(format!(
                    "selector {selector} not found in time"
                )));
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }

    /// Get the text of the first element matching a CSS selector.
    pub fn get_element(&self, target: &Target, selector: &str) -> Result<String> {
        let expr = format!(
            "(() => {{ const e = document.querySelector({}); return e ? e.innerText : ''; }})()",
            serde_json::to_string(selector)?
        );
        self.evaluate(target, &expr)
    }

    /// Scroll the page by a delta.
    pub fn scroll(&self, target: &Target, dx: i32, dy: i32) -> Result<()> {
        let expr = format!("window.scrollBy({dx}, {dy})");
        self.evaluate(target, &expr)?;
        Ok(())
    }

    /// Get the page's cookies.
    pub fn get_cookies(&self, target: &Target) -> Result<String> {
        let result = self.send_command(target, "Network.getCookies", json!({}))?;
        let cookies = result
            .get("cookies")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut lines = Vec::new();
        for cookie in cookies {
            let name = cookie.get("name").and_then(Value::as_str).unwrap_or("");
            let value = cookie.get("value").and_then(Value::as_str).unwrap_or("");
            lines.push(format!("{name}={value}"));
        }
        Ok(lines.join("\n"))
    }

    /// Set a cookie on the page.
    pub fn set_cookie(&self, target: &Target, name: &str, value: &str, url: &str) -> Result<()> {
        self.send_command(
            target,
            "Network.setCookie",
            json!({
                "name": name, "value": value, "url": url
            }),
        )?;
        Ok(())
    }

    /// Get the page's localStorage as JSON.
    pub fn get_local_storage(&self, target: &Target) -> Result<String> {
        self.evaluate(target, "JSON.stringify(localStorage)")
    }

    /// Set a localStorage key.
    pub fn set_local_storage(&self, target: &Target, key: &str, value: &str) -> Result<()> {
        let expr = format!(
            "localStorage.setItem({}, {})",
            serde_json::to_string(key)?,
            serde_json::to_string(value)?
        );
        self.evaluate(target, &expr)?;
        Ok(())
    }

    /// Get navigation performance metrics as JSON.
    pub fn get_performance(&self, target: &Target) -> Result<String> {
        self.evaluate(
            target,
            "JSON.stringify(performance.getEntriesByType('navigation')[0] || {})",
        )
    }

    /// Get an attribute of the first element matching a selector.
    pub fn get_attribute(&self, target: &Target, selector: &str, attr: &str) -> Result<String> {
        let expr = format!(
            "(() => {{ const e = document.querySelector({}); return e ? (e.getAttribute({}) || '') : ''; }})()",
            serde_json::to_string(selector)?,
            serde_json::to_string(attr)?
        );
        self.evaluate(target, &expr)
    }

    /// Set an attribute on the first element matching a selector.
    pub fn set_attribute(
        &self,
        target: &Target,
        selector: &str,
        attr: &str,
        value: &str,
    ) -> Result<()> {
        let expr = format!(
            "(() => {{ const e = document.querySelector({}); if (e) e.setAttribute({}, {}); }})()",
            serde_json::to_string(selector)?,
            serde_json::to_string(attr)?,
            serde_json::to_string(value)?
        );
        self.evaluate(target, &expr)?;
        Ok(())
    }

    /// Get the value of the first element matching a selector.
    pub fn get_value(&self, target: &Target, selector: &str) -> Result<String> {
        let expr = format!(
            "(() => {{ const e = document.querySelector({}); return e ? (e.value || '') : ''; }})()",
            serde_json::to_string(selector)?
        );
        self.evaluate(target, &expr)
    }

    /// Set the value of the first element matching a selector.
    pub fn set_value(&self, target: &Target, selector: &str, value: &str) -> Result<()> {
        let expr = format!(
            "(() => {{ const e = document.querySelector({}); if (e) {{ e.value = {}; e.dispatchEvent(new Event('input', {{ bubbles: true }})); }} }})()",
            serde_json::to_string(selector)?,
            serde_json::to_string(value)?
        );
        self.evaluate(target, &expr)?;
        Ok(())
    }

    /// Focus the first element matching a selector.
    pub fn focus(&self, target: &Target, selector: &str) -> Result<()> {
        let expr = format!(
            "(() => {{ const e = document.querySelector({}); if (e) e.focus(); }})()",
            serde_json::to_string(selector)?
        );
        self.evaluate(target, &expr)?;
        Ok(())
    }

    /// Get the bounding rect of the first element matching a selector.
    pub fn get_rect(&self, target: &Target, selector: &str) -> Result<String> {
        let expr = format!(
            "(() => {{ const e = document.querySelector({}); if (!e) return '{{}}'; const r = e.getBoundingClientRect(); return JSON.stringify({{x: r.x, y: r.y, width: r.width, height: r.height}}); }})()",
            serde_json::to_string(selector)?
        );
        self.evaluate(target, &expr)
    }

    /// List the links on the page as `text -> href` lines.
    pub fn get_links(&self, target: &Target) -> Result<String> {
        self.evaluate(
            target,
            "Array.from(document.querySelectorAll('a')).map(a => (a.innerText.trim() + ' -> ' + a.href)).join('\\n')",
        )
    }

    /// List the images on the page as `alt -> src` lines.
    pub fn get_images(&self, target: &Target) -> Result<String> {
        self.evaluate(
            target,
            "Array.from(document.querySelectorAll('img')).map(i => (i.alt + ' -> ' + i.src)).join('\\n')",
        )
    }

    /// List the forms on the page.
    pub fn get_forms(&self, target: &Target) -> Result<String> {
        self.evaluate(
            target,
            "Array.from(document.querySelectorAll('form')).map((f, i) => i + ': action=' + f.action + ' method=' + f.method).join('\\n')",
        )
    }

    /// Get the page's meta tags.
    pub fn get_meta(&self, target: &Target) -> Result<String> {
        self.evaluate(
            target,
            "Array.from(document.querySelectorAll('meta')).map(m => (m.getAttribute('name') || m.getAttribute('property') || '') + '=' + (m.getAttribute('content') || '')).filter(s => s.startsWith('=') === false).join('\\n')",
        )
    }

    /// Get the visible text of the page (only visible elements).
    pub fn get_visible_text(&self, target: &Target) -> Result<String> {
        self.evaluate(
            target,
            "(() => { const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT); const parts = []; while (walker.nextNode()) { const n = walker.currentNode; if (n.parentElement && n.parentElement.offsetParent !== null) parts.push(n.textContent); } return parts.join(' '); })()",
        )
    }

    /// Get the outer HTML of the first element matching a selector.
    pub fn get_outer_html(&self, target: &Target, selector: &str) -> Result<String> {
        let expr = format!(
            "(() => {{ const e = document.querySelector({}); return e ? e.outerHTML : ''; }})()",
            serde_json::to_string(selector)?
        );
        self.evaluate(target, &expr)
    }

    /// Get the inner HTML of the first element matching a selector.
    pub fn get_inner_html(&self, target: &Target, selector: &str) -> Result<String> {
        let expr = format!(
            "(() => {{ const e = document.querySelector({}); return e ? e.innerHTML : ''; }})()",
            serde_json::to_string(selector)?
        );
        self.evaluate(target, &expr)
    }

    /// List the direct children of the first element matching a selector.
    pub fn get_children(&self, target: &Target, selector: &str) -> Result<String> {
        let expr = format!(
            "(() => {{ const e = document.querySelector({}); if (!e) return ''; return Array.from(e.children).map(c => c.tagName + '#' + (c.id || '') + '.' + (c.className || '')).join('\\n'); }})()",
            serde_json::to_string(selector)?
        );
        self.evaluate(target, &expr)
    }

    /// Get the parent of the first element matching a selector.
    pub fn get_parent(&self, target: &Target, selector: &str) -> Result<String> {
        let expr = format!(
            "(() => {{ const e = document.querySelector({}); if (!e || !e.parentElement) return ''; const p = e.parentElement; return p.tagName + '#' + (p.id || '') + '.' + (p.className || ''); }})()",
            serde_json::to_string(selector)?
        );
        self.evaluate(target, &expr)
    }

    /// Get the class list of the first element matching a selector.
    pub fn get_class_list(&self, target: &Target, selector: &str) -> Result<String> {
        let expr = format!(
            "(() => {{ const e = document.querySelector({}); return e ? e.className : ''; }})()",
            serde_json::to_string(selector)?
        );
        self.evaluate(target, &expr)
    }

    /// Get the inline style of the first element matching a selector.
    pub fn get_style(&self, target: &Target, selector: &str) -> Result<String> {
        let expr = format!(
            "(() => {{ const e = document.querySelector({}); return e ? e.getAttribute('style') || '' : ''; }})()",
            serde_json::to_string(selector)?
        );
        self.evaluate(target, &expr)
    }

    /// Get whether the first element matching a selector is checked.
    pub fn get_checked(&self, target: &Target, selector: &str) -> Result<String> {
        let expr = format!(
            "(() => {{ const e = document.querySelector({}); return e ? String(e.checked) : ''; }})()",
            serde_json::to_string(selector)?
        );
        self.evaluate(target, &expr)
    }

    /// Get the placeholder of the first element matching a selector.
    pub fn get_placeholder(&self, target: &Target, selector: &str) -> Result<String> {
        let expr = format!(
            "(() => {{ const e = document.querySelector({}); return e ? (e.placeholder || '') : ''; }})()",
            serde_json::to_string(selector)?
        );
        self.evaluate(target, &expr)
    }

    /// Get the href of the first element matching a selector.
    pub fn get_href(&self, target: &Target, selector: &str) -> Result<String> {
        let expr = format!(
            "(() => {{ const e = document.querySelector({}); return e ? (e.href || '') : ''; }})()",
            serde_json::to_string(selector)?
        );
        self.evaluate(target, &expr)
    }

    /// Get the src of the first element matching a selector.
    pub fn get_src(&self, target: &Target, selector: &str) -> Result<String> {
        let expr = format!(
            "(() => {{ const e = document.querySelector({}); return e ? (e.src || '') : ''; }})()",
            serde_json::to_string(selector)?
        );
        self.evaluate(target, &expr)
    }

    /// Get the alt text of the first element matching a selector.
    pub fn get_alt(&self, target: &Target, selector: &str) -> Result<String> {
        let expr = format!(
            "(() => {{ const e = document.querySelector({}); return e ? (e.alt || '') : ''; }})()",
            serde_json::to_string(selector)?
        );
        self.evaluate(target, &expr)
    }

    /// List the form input fields on the page.
    pub fn get_form_fields(&self, target: &Target) -> Result<String> {
        self.evaluate(
            target,
            "Array.from(document.querySelectorAll('input, textarea, select')).map(e => (e.name || e.id || '') + '=' + (e.value || '')).join('\\n')",
        )
    }

    /// List the resource URLs the page has loaded (from performance entries).
    pub fn get_network_requests(&self, target: &Target) -> Result<String> {
        self.evaluate(
            target,
            "performance.getEntriesByType('resource').map(e => e.name).join('\\n')",
        )
    }

    /// Get the page's response headers by fetching the current URL with HEAD.
    pub fn get_headers(&self, target: &Target) -> Result<String> {
        self.evaluate(
            target,
            "fetch(location.href, {method: 'HEAD'}).then(r => { const h = {}; r.headers.forEach((v, k) => h[k] = v); return JSON.stringify(h); })",
        )
    }

    /// Read console messages by enabling the Runtime domain and collecting
    /// `consoleAPICalled` events for a short window.
    pub fn get_console_messages(&self, target: &Target) -> Result<String> {
        let (mut socket, _) = tungstenite::connect(&target.ws_url)
            .map_err(|e| Error::Agent(format!("connect devtools: {e}")))?;
        socket
            .send(tungstenite::Message::Text(serde_json::to_string(&json!({
                "id": 1, "method": "Runtime.enable", "params": {}
            }))?))
            .map_err(|e| Error::Agent(format!("enable runtime: {e}")))?;
        // Trigger a console message so we have something to read.
        socket
            .send(tungstenite::Message::Text(serde_json::to_string(&json!({
                "id": 2, "method": "Runtime.evaluate",
                "params": { "expression": "console.log('forge-probe')" }
            }))?))
            .map_err(|e| Error::Agent(format!("probe console: {e}")))?;
        let mut messages = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
        while std::time::Instant::now() < deadline {
            match socket.read() {
                Ok(tungstenite::Message::Text(t)) => {
                    if let Ok(value) = serde_json::from_str::<Value>(&t) {
                        if value.get("method").and_then(Value::as_str)
                            == Some("Runtime.consoleAPICalled")
                        {
                            if let Some(args) =
                                value.pointer("/params/args").and_then(Value::as_array)
                            {
                                let text: Vec<String> = args
                                    .iter()
                                    .filter_map(|a| {
                                        a.get("value").and_then(Value::as_str).map(str::to_string)
                                    })
                                    .collect();
                                messages.push(text.join(" "));
                            }
                        }
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        if messages.is_empty() {
            Ok("no console messages".into())
        } else {
            Ok(messages.join("\n"))
        }
    }

    /// Click at a coordinate in the target.
    pub fn click(&self, target: &Target, x: i32, y: i32) -> Result<()> {
        self.send_command(
            target,
            "Input.dispatchMouseEvent",
            json!({
                "type": "mousePressed", "x": x, "y": y, "button": "left", "clickCount": 1
            }),
        )?;
        self.send_command(
            target,
            "Input.dispatchMouseEvent",
            json!({
                "type": "mouseReleased", "x": x, "y": y, "button": "left", "clickCount": 1
            }),
        )?;
        Ok(())
    }

    /// Type text into the focused element of the target.
    pub fn type_text(&self, target: &Target, text: &str) -> Result<()> {
        self.send_command(target, "Input.insertText", json!({ "text": text }))?;
        Ok(())
    }

    /// Capture a screenshot of the target as a PNG, returning the base64 data.
    pub fn screenshot(&self, target: &Target) -> Result<String> {
        let result =
            self.send_command(target, "Page.captureScreenshot", json!({ "format": "png" }))?;
        result
            .get("data")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| Error::Agent("no screenshot data".into()))
    }

    /// Send a CDP command over the target's WebSocket and return the result.
    fn send_command(&self, target: &Target, method: &str, params: Value) -> Result<Value> {
        let (mut socket, _) = tungstenite::connect(&target.ws_url)
            .map_err(|e| Error::Agent(format!("connect devtools: {e}")))?;
        let request = json!({ "id": 1, "method": method, "params": params });
        let message = tungstenite::Message::Text(serde_json::to_string(&request)?);
        socket
            .send(message)
            .map_err(|e| Error::Agent(format!("send {method}: {e}")))?;
        let reply = socket
            .read()
            .map_err(|e| Error::Agent(format!("read {method}: {e}")))?;
        let text = match reply {
            tungstenite::Message::Text(t) => t.to_string(),
            tungstenite::Message::Binary(b) => String::from_utf8_lossy(&b).into_owned(),
            _ => return Err(Error::Agent("unexpected devtools message".into())),
        };
        let value: Value = serde_json::from_str(&text)
            .map_err(|e| Error::Agent(format!("parse {method}: {e}")))?;
        if let Some(err) = value.get("error") {
            return Err(Error::Agent(format!("CDP error: {err}")));
        }
        value
            .get("result")
            .cloned()
            .ok_or_else(|| Error::Agent(format!("no result for {method}")))
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

    /// List targets as (id, url) pairs.
    pub fn list_targets(&self) -> Result<Vec<(String, String)>> {
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
                    .filter_map(|t| {
                        let id = t.get("id").and_then(Value::as_str)?;
                        let url = t.get("url").and_then(Value::as_str).unwrap_or("");
                        Some((id.to_string(), url.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    /// Close a target by id.
    pub fn close_target(&self, id: &str) -> Result<()> {
        let endpoint = format!("http://127.0.0.1:{}/json/close/{}", self.port, id);
        self.client
            .get(&endpoint)
            .send()
            .map_err(|e| Error::Agent(format!("close target: {e}")))?;
        Ok(())
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
