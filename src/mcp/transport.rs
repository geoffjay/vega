//! # MCP Transport Layer
//!
//! This module provides transport layer abstractions for MCP communication.
//! It supports various transport mechanisms including stdio and SSE.

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use rust_mcp_schema::{JsonrpcMessage, JsonrpcRequest, JsonrpcResponse, RequestParams};
use serde_json::Value;
use std::process::{Child, Stdio};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};

use super::config::{TransportConfig, TransportType};

/// Trait for MCP transport implementations
#[async_trait]
pub trait McpTransport: Send + Sync {
    /// Send a message to the remote endpoint
    async fn send(&mut self, message: JsonrpcMessage) -> Result<()>;

    /// Receive a message from the remote endpoint
    async fn receive(&mut self) -> Result<JsonrpcMessage>;

    /// Close the transport connection
    async fn close(&mut self) -> Result<()>;

    /// Check if the transport is connected
    fn is_connected(&self) -> bool;
}

/// Factory for creating transport instances
pub struct TransportFactory;

impl TransportFactory {
    /// Create a new transport based on the configuration
    pub fn create(config: TransportConfig) -> Result<Box<dyn McpTransport>> {
        match config.transport_type {
            TransportType::Stdio => Ok(Box::new(StdioTransport::new(config)?)),
            TransportType::Sse => Err(anyhow!("SSE transport not yet implemented")),
            TransportType::Http => Err(anyhow!("HTTP transport not yet implemented")),
        }
    }
}

/// Stdio-based transport implementation
pub struct StdioTransport {
    child: Option<Child>,
    sender: Option<mpsc::UnboundedSender<McpMessage>>,
    receiver: Option<mpsc::UnboundedReceiver<McpMessage>>,
    connected: bool,
    timeout_duration: Duration,
}

impl StdioTransport {
    /// Create a new stdio transport
    pub fn new(config: TransportConfig) -> Result<Self> {
        let timeout_duration = Duration::from_secs(config.options.timeout.unwrap_or(30));

        Ok(Self {
            child: None,
            sender: None,
            receiver: None,
            connected: false,
            timeout_duration,
        })
    }

    /// Start a child process and establish stdio communication
    pub async fn connect(&mut self, command: &str, args: &[String]) -> Result<()> {
        // Start the child process
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!("Failed to start MCP server process: {}", e))?;

        // Get stdin and stdout handles
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Failed to get stdin handle"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Failed to get stdout handle"))?;

        // Create channels for communication
        let (tx, rx) = mpsc::unbounded_channel();
        let (response_tx, response_rx) = mpsc::unbounded_channel();

        // Start the writer task
        let timeout_duration = self.timeout_duration;
        tokio::spawn(async move {
            let mut stdin = stdin;
            let mut rx = rx;

            while let Some(message) = rx.recv().await {
                if let Ok(json) = serde_json::to_string(&message) {
                    let line = format!("{}\n", json);
                    if let Err(e) = stdin.write_all(line.as_bytes()).await {
                        tracing::error!("Failed to write to MCP server stdin: {}", e);
                        break;
                    }
                    if let Err(e) = stdin.flush().await {
                        tracing::error!("Failed to flush MCP server stdin: {}", e);
                        break;
                    }
                }
            }
        });

        // Start the reader task
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(message) = serde_json::from_str::<McpMessage>(&line) {
                    if let Err(_) = response_tx.send(message) {
                        break; // Channel closed
                    }
                } else {
                    tracing::warn!("Failed to parse MCP message: {}", line);
                }
            }
        });

        self.child = Some(child);
        self.sender = Some(tx);
        self.receiver = Some(response_rx);
        self.connected = true;

        Ok(())
    }
}

#[async_trait]
impl McpTransport for StdioTransport {
    async fn send(&mut self, message: McpMessage) -> Result<()> {
        if !self.connected {
            return Err(anyhow!("Transport not connected"));
        }

        if let Some(sender) = &self.sender {
            sender
                .send(message)
                .map_err(|e| anyhow!("Failed to send message: {}", e))?;
            Ok(())
        } else {
            Err(anyhow!("Sender not available"))
        }
    }

    async fn receive(&mut self) -> Result<McpMessage> {
        if !self.connected {
            return Err(anyhow!("Transport not connected"));
        }

        if let Some(receiver) = &mut self.receiver {
            timeout(self.timeout_duration, receiver.recv())
                .await
                .map_err(|_| anyhow!("Timeout waiting for message"))?
                .ok_or_else(|| anyhow!("Channel closed"))
        } else {
            Err(anyhow!("Receiver not available"))
        }
    }

    async fn close(&mut self) -> Result<()> {
        self.connected = false;

        // Close channels
        self.sender = None;
        self.receiver = None;

        // Terminate child process
        if let Some(mut child) = self.child.take() {
            if let Err(e) = child.kill().await {
                tracing::warn!("Failed to kill child process: {}", e);
            }
            if let Err(e) = child.wait().await {
                tracing::warn!("Failed to wait for child process: {}", e);
            }
        }

        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

/// Message router for handling MCP request/response correlation
#[derive(Debug)]
pub struct MessageRouter {
    pending_requests: std::collections::HashMap<u64, tokio::sync::oneshot::Sender<JsonrpcResponse>>,
    next_id: u64,
}

impl MessageRouter {
    /// Create a new message router
    pub fn new() -> Self {
        Self {
            pending_requests: std::collections::HashMap::new(),
            next_id: 1,
        }
    }

    /// Generate a unique request ID
    pub fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Register a pending request
    pub fn register_request(&mut self, id: u64) -> tokio::sync::oneshot::Receiver<JsonrpcResponse> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending_requests.insert(id, tx);
        rx
    }

    /// Handle an incoming response
    pub fn handle_response(&mut self, response: JsonrpcResponse) {
        // Extract ID from the response and handle appropriately
        if let Some(tx) = self.pending_requests.remove(&id) {
            let _ = tx.send(response);
        }
    }

    /// Clean up expired requests
    pub fn cleanup_expired(&mut self) {
        // TODO: Implement cleanup based on timestamps
    }
}

/// Helper for building MCP requests
pub struct RequestBuilder;

impl RequestBuilder {
    /// Build a list_tools request
    pub fn list_tools(id: u64) -> Request {
        Request {
            id: Value::Number(id.into()),
            method: "tools/list".to_string(),
            params: None,
        }
    }

    /// Build a call_tool request
    pub fn call_tool(id: u64, name: &str, arguments: Option<Value>) -> Request {
        let mut params = serde_json::Map::new();
        params.insert("name".to_string(), Value::String(name.to_string()));
        if let Some(args) = arguments {
            params.insert("arguments".to_string(), args);
        }

        Request {
            id: Value::Number(id.into()),
            method: "tools/call".to_string(),
            params: Some(Value::Object(params)),
        }
    }

    /// Build a list_resources request
    pub fn list_resources(id: u64) -> Request {
        Request {
            id: Value::Number(id.into()),
            method: "resources/list".to_string(),
            params: None,
        }
    }

    /// Build a read_resource request
    pub fn read_resource(id: u64, uri: &str) -> Request {
        let mut params = serde_json::Map::new();
        params.insert("uri".to_string(), Value::String(uri.to_string()));

        Request {
            id: Value::Number(id.into()),
            method: "resources/read".to_string(),
            params: Some(Value::Object(params)),
        }
    }

    /// Build an initialize request
    pub fn initialize(id: u64, client_info: Value) -> Request {
        Request {
            id: Value::Number(id.into()),
            method: "initialize".to_string(),
            params: Some(client_info),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_router() {
        let mut router = MessageRouter::new();

        let id1 = router.next_id();
        let id2 = router.next_id();

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);

        let _rx1 = router.register_request(id1);
        let _rx2 = router.register_request(id2);

        assert_eq!(router.pending_requests.len(), 2);
    }

    #[test]
    fn test_request_builder() {
        let request = RequestBuilder::list_tools(1);
        assert_eq!(request.method, "tools/list");
        assert_eq!(request.id, Value::Number(1.into()));

        let request = RequestBuilder::call_tool(2, "test_tool", None);
        assert_eq!(request.method, "tools/call");
        assert_eq!(request.id, Value::Number(2.into()));
    }
}
