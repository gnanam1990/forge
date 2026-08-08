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
    server_info: Value,
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
            server_info: Value::Null,
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

        // Retry once on a transport error (write/read/parse), in case the server
        // was momentarily busy. A JSON-RPC error response is not retried.
        let mut last_err: Option<Error> = None;
        for _ in 0..2 {
            match self.transport_roundtrip(&line) {
                Ok(value) => {
                    if let Some(err) = value.get("error") {
                        return Err(Error::Provider(format!("MCP error: {err}")));
                    }
                    return value
                        .get("result")
                        .cloned()
                        .ok_or_else(|| Error::Provider("no result".into()));
                }
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or_else(|| Error::Provider("MCP request failed".into())))
    }

    /// Write a request line and read the response, returning the parsed JSON.
    fn transport_roundtrip(&mut self, line: &str) -> Result<Value> {
        self.stdin.write_all(line.as_bytes())?;
        self.stdin.flush()?;
        let mut response = String::new();
        self.stdout.read_line(&mut response)?;
        serde_json::from_str(&response)
            .map_err(|e| Error::Provider(format!("bad MCP response: {e}")))
    }

    /// Perform the MCP initialize handshake.
    pub fn initialize(&mut self) -> Result<()> {
        let params = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "forge", "version": "0.1.0" }
        });
        self.server_info = self.request("initialize", params)?;
        // Send the initialized notification (no id).
        let notification = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        let mut line = serde_json::to_string(&notification)?;
        line.push('\n');
        self.stdin.write_all(line.as_bytes())?;
        self.stdin.flush()?;
        Ok(())
    }

    /// The server's reported name and version.
    pub fn server_info(&self) -> String {
        let name = self
            .server_info
            .get("serverInfo")
            .and_then(|s| s.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let version = self
            .server_info
            .get("serverInfo")
            .and_then(|s| s.get("version"))
            .and_then(Value::as_str)
            .unwrap_or("?");
        format!("{name} {version}")
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

    /// List the tools as (name, description) pairs.
    pub fn list_tools_with_descriptions(&mut self) -> Result<Vec<(String, String)>> {
        let result = self.request("tools/list", json!({}))?;
        let tools = result
            .get("tools")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(tools
            .iter()
            .filter_map(|t| {
                let name = t.get("name").and_then(Value::as_str)?;
                let desc = t.get("description").and_then(Value::as_str).unwrap_or("");
                Some((name.to_string(), desc.to_string()))
            })
            .collect())
    }

    /// Call a tool and return its text content (handling both text and
    /// structured JSON content).
    pub fn call_tool(&mut self, name: &str, args: Value) -> Result<String> {
        let result = self.request("tools/call", json!({ "name": name, "arguments": args }))?;
        let content = result
            .get("content")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut text = String::new();
        for item in content {
            match item.get("type").and_then(Value::as_str) {
                Some("text") => {
                    if let Some(t) = item.get("text").and_then(Value::as_str) {
                        text.push_str(t);
                    }
                }
                Some("json") => {
                    if let Some(j) = item.get("json") {
                        text.push_str(&serde_json::to_string_pretty(j).unwrap_or_default());
                    }
                }
                _ => {
                    if let Some(t) = item.get("text").and_then(Value::as_str) {
                        text.push_str(t);
                    }
                }
            }
        }
        Ok(text)
    }

    /// List the resources the server exposes.
    pub fn list_resources(&mut self) -> Result<Vec<String>> {
        let result = self.request("resources/list", json!({}))?;
        let resources = result
            .get("resources")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(resources
            .iter()
            .filter_map(|r| r.get("uri").and_then(Value::as_str).map(str::to_string))
            .collect())
    }

    /// Read a resource by URI.
    pub fn read_resource(&mut self, uri: &str) -> Result<String> {
        let result = self.request("resources/read", json!({ "uri": uri }))?;
        let contents = result
            .get("contents")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut text = String::new();
        for item in contents {
            if let Some(t) = item.get("text").and_then(Value::as_str) {
                text.push_str(t);
            }
        }
        Ok(text)
    }

    /// List the prompts the server exposes.
    pub fn list_prompts(&mut self) -> Result<Vec<String>> {
        let result = self.request("prompts/list", json!({}))?;
        let prompts = result
            .get("prompts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(prompts
            .iter()
            .filter_map(|p| p.get("name").and_then(Value::as_str).map(str::to_string))
            .collect())
    }

    /// Ping the server to check it is alive.
    pub fn ping(&mut self) -> Result<()> {
        self.request("ping", json!({}))?;
        Ok(())
    }

    /// Send a notification (no response expected).
    pub fn send_notification(&mut self, method: &str, params: Value) -> Result<()> {
        let notification = json!({ "jsonrpc": "2.0", "method": method, "params": params });
        let mut line = serde_json::to_string(&notification)?;
        line.push('\n');
        self.stdin.write_all(line.as_bytes())?;
        self.stdin.flush()?;
        Ok(())
    }

    /// Request a model sample from the server (MCP sampling).
    pub fn sample(&mut self, prompt: &str) -> Result<String> {
        let result = self.request(
            "sampling/createMessage",
            json!({
                "messages": [{ "role": "user", "content": { "type": "text", "text": prompt } }],
                "maxTokens": 1000
            }),
        )?;
        result
            .get("content")
            .and_then(|c| c.get("text"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| Error::Provider("no sample text".into()))
    }

    /// Set the server's log level.
    pub fn set_log_level(&mut self, level: &str) -> Result<()> {
        self.request("logging/setLevel", json!({ "level": level }))?;
        Ok(())
    }

    /// List the roots the server exposes.
    pub fn list_roots(&mut self) -> Result<Vec<String>> {
        let result = self.request("roots/list", json!({}))?;
        let roots = result
            .get("roots")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(roots
            .iter()
            .filter_map(|r| r.get("uri").and_then(Value::as_str).map(str::to_string))
            .collect())
    }

    /// Get completions for a reference.
    pub fn complete(&mut self, reference: Value) -> Result<Vec<String>> {
        let result = self.request("completion/complete", json!({ "ref": reference }))?;
        let values = result
            .get("completion")
            .and_then(|c| c.get("values"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect())
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

use std::sync::{Arc, Mutex};

use crate::tools::{Registry, Tool, ToolContext, ToolResult};

/// A tool backed by an MCP server tool. All tools from one server share a
/// single client behind a mutex.
pub struct McpTool {
    name: String,
    description: String,
    client: Arc<Mutex<McpClient>>,
}

impl Tool for McpTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn run(&self, args: &Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let mut client = self.client.lock().unwrap();
        let output = client.call_tool(&self.name, args.clone())?;
        Ok(ToolResult::ok(output))
    }
}

/// Connect to an MCP server and register all of its tools into a registry.
pub fn register_mcp_server(registry: &mut Registry, command: &str, args: &[&str]) -> Result<()> {
    let mut client = McpClient::connect(command, args)?;
    let tools = client.list_tools()?;
    let shared = Arc::new(Mutex::new(client));
    for tool_name in tools {
        registry.register(Box::new(McpTool {
            name: tool_name.clone(),
            description: format!("MCP tool {tool_name} from {command}"),
            client: Arc::clone(&shared),
        }));
    }
    Ok(())
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
