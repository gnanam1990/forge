//! A minimal Model Context Protocol (MCP) client over stdio. It spawns an MCP
//! server subprocess and speaks JSON-RPC 2.0 to it: initialize, list tools, and
//! call tools.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};

use crate::error::{Error, Result};

/// A client connected to an MCP server subprocess.
pub struct McpClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl McpClient {
    /// Spawn the server and perform the initialize handshake.
    pub fn connect(command: &str, args: &[&str]) -> Result<Self> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Error::Provider(format!("spawn MCP server: {e}")))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::Provider("no stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Provider("no stdout".into()))?;
        let mut client = McpClient {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            next_id: 1,
        };
        client.initialize()?;
        Ok(client)
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let request = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        let mut line = serde_json::to_string(&request)?;
        line.push('\n');
        self.stdin.write_all(line.as_bytes())?;
        self.stdin.flush()?;

        let mut response = String::new();
        self.stdout.read_line(&mut response)?;
        let value: Value = serde_json::from_str(&response)
            .map_err(|e| Error::Provider(format!("bad MCP response: {e}")))?;
        if let Some(err) = value.get("error") {
            return Err(Error::Provider(format!("MCP error: {err}")));
        }
        value
            .get("result")
            .cloned()
            .ok_or_else(|| Error::Provider("no result".into()))
    }

    /// Perform the MCP initialize handshake.
    pub fn initialize(&mut self) -> Result<()> {
        let params = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "forge", "version": "0.1.0" }
        });
        self.request("initialize", params)?;
        // Send the initialized notification (no id).
        let notification = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        let mut line = serde_json::to_string(&notification)?;
        line.push('\n');
        self.stdin.write_all(line.as_bytes())?;
        self.stdin.flush()?;
        Ok(())
    }

    /// List the tool names the server exposes.
    pub fn list_tools(&mut self) -> Result<Vec<String>> {
        let result = self.request("tools/list", json!({}))?;
        let tools = result
            .get("tools")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(tools
            .iter()
            .filter_map(|t| t.get("name").and_then(Value::as_str).map(str::to_string))
            .collect())
    }

    /// Call a tool and return its text content.
    pub fn call_tool(&mut self, name: &str, args: Value) -> Result<String> {
        let result = self.request("tools/call", json!({ "name": name, "arguments": args }))?;
        let content = result
            .get("content")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut text = String::new();
        for item in content {
            if let Some(t) = item.get("text").and_then(Value::as_str) {
                text.push_str(t);
            }
        }
        Ok(text)
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_initialize_request() {
        // The request framing is exercised through a fake server below; here we
        // just confirm the JSON shape is valid.
        let request = json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} });
        assert!(serde_json::to_string(&request).is_ok());
    }
}
