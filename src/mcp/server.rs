//! # MCP Server Implementation
//!
//! This module provides MCP server functionality, allowing Vega to expose its tools
//! as MCP tools for other AI systems to consume.

use anyhow::{Result, anyhow};
use rust_mcp_schema::{ErrorCode, McpError, McpMessage, Request, Response, Tool as McpToolDef};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use super::bridge::{McpToolFactory, VegaToMcpBridge};
use super::config::McpServerConfig;
use crate::tools::{
    BashTool, CodeSearchTool, EditFileTool, ListFilesTool, ReadFileTool, ReadLogsTool, RigTool,
    WebSearchTool,
};

/// MCP server that exposes Vega's tools
#[derive(Debug)]
pub struct McpServer {
    /// Server configuration
    config: McpServerConfig,
    /// Available tools bridge
    bridge: Arc<RwLock<VegaToMcpBridge>>,
    /// Server capabilities
    capabilities: ServerCapabilities,
    /// Running state
    running: Arc<RwLock<bool>>,
    /// Server task handle
    task_handle: Option<JoinHandle<Result<()>>>,
}

/// Server capabilities structure
#[derive(Debug, Clone)]
pub struct ServerCapabilities {
    /// Tools capabilities
    pub tools: Option<ToolsCapability>,
    /// Resources capabilities (not implemented yet)
    pub resources: Option<ResourcesCapability>,
    /// Prompts capabilities (not implemented yet)
    pub prompts: Option<PromptsCapability>,
    /// Logging capabilities
    pub logging: Option<LoggingCapability>,
}

/// Tools capability
#[derive(Debug, Clone)]
pub struct ToolsCapability {
    /// Supports list_changed notifications
    pub list_changed: bool,
}

/// Resources capability (placeholder)
#[derive(Debug, Clone)]
pub struct ResourcesCapability {
    /// Supports subscribe to resource changes
    pub subscribe: bool,
    /// Supports list_changed notifications
    pub list_changed: bool,
}

/// Prompts capability (placeholder)
#[derive(Debug, Clone)]
pub struct PromptsCapability {
    /// Supports list_changed notifications
    pub list_changed: bool,
}

/// Logging capability
#[derive(Debug, Clone)]
pub struct LoggingCapability {
    /// Supported log levels
    pub levels: Vec<String>,
}

impl Default for ServerCapabilities {
    fn default() -> Self {
        Self {
            tools: Some(ToolsCapability {
                list_changed: false, // We don't currently support dynamic tool changes
            }),
            resources: None, // Not implemented yet
            prompts: None,   // Not implemented yet
            logging: Some(LoggingCapability {
                levels: vec![
                    "error".to_string(),
                    "warn".to_string(),
                    "info".to_string(),
                    "debug".to_string(),
                ],
            }),
        }
    }
}

impl McpServer {
    /// Create a new MCP server
    pub async fn new(config: McpServerConfig) -> Result<Self> {
        let mut bridge = VegaToMcpBridge::new();

        // Add Vega tools to the bridge based on configuration
        Self::setup_tools(&mut bridge, &config.exposed_tools).await?;

        Ok(Self {
            config,
            bridge: Arc::new(RwLock::new(bridge)),
            capabilities: ServerCapabilities::default(),
            running: Arc::new(RwLock::new(false)),
            task_handle: None,
        })
    }

    /// Setup tools in the bridge based on configuration
    async fn setup_tools(bridge: &mut VegaToMcpBridge, exposed_tools: &[String]) -> Result<()> {
        for tool_name in exposed_tools {
            match tool_name.as_str() {
                "bash" => {
                    let tool = Box::new(BashTool::new()) as Box<dyn RigTool>;
                    bridge.add_tool("bash".to_string(), tool);
                }
                "read_file" => {
                    let tool = Box::new(ReadFileTool::new()) as Box<dyn RigTool>;
                    bridge.add_tool("read_file".to_string(), tool);
                }
                "edit_file" => {
                    let tool = Box::new(EditFileTool::new()) as Box<dyn RigTool>;
                    bridge.add_tool("edit_file".to_string(), tool);
                }
                "list_files" => {
                    let tool = Box::new(ListFilesTool::new()) as Box<dyn RigTool>;
                    bridge.add_tool("list_files".to_string(), tool);
                }
                "code_search" => {
                    let tool = Box::new(CodeSearchTool::new()) as Box<dyn RigTool>;
                    bridge.add_tool("code_search".to_string(), tool);
                }
                "web_search" => {
                    let tool = Box::new(WebSearchTool::new()) as Box<dyn RigTool>;
                    bridge.add_tool("web_search".to_string(), tool);
                }
                "read_logs" => {
                    let tool = Box::new(ReadLogsTool::new()) as Box<dyn RigTool>;
                    bridge.add_tool("read_logs".to_string(), tool);
                }
                _ => {
                    tracing::warn!("Unknown tool '{}' in configuration", tool_name);
                }
            }
        }

        tracing::info!("Configured {} tools for MCP server", exposed_tools.len());
        Ok(())
    }

    /// Start the MCP server
    pub async fn run(mut self) -> Result<()> {
        *self.running.write().await = true;

        let bridge = self.bridge.clone();
        let capabilities = self.capabilities.clone();
        let running = self.running.clone();

        let handle =
            tokio::spawn(async move { Self::serve_stdio(bridge, capabilities, running).await });

        self.task_handle = Some(handle);

        // Wait for the server task to complete
        if let Some(handle) = self.task_handle {
            handle.await??;
        }

        Ok(())
    }

    /// Serve over stdio (JSON-RPC over stdin/stdout)
    async fn serve_stdio(
        bridge: Arc<RwLock<VegaToMcpBridge>>,
        capabilities: ServerCapabilities,
        running: Arc<RwLock<bool>>,
    ) -> Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        tracing::info!("MCP server started, listening on stdio");

        while *running.read().await {
            line.clear();

            match reader.read_line(&mut line).await {
                Ok(0) => {
                    // EOF reached
                    tracing::info!("EOF reached, shutting down MCP server");
                    break;
                }
                Ok(_) => {
                    // Process the line
                    if let Err(e) =
                        Self::process_message(&line, &mut stdout, &bridge, &capabilities).await
                    {
                        tracing::error!("Error processing message: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("Error reading from stdin: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Process a single MCP message
    async fn process_message(
        line: &str,
        stdout: &mut io::Stdout,
        bridge: &Arc<RwLock<VegaToMcpBridge>>,
        capabilities: &ServerCapabilities,
    ) -> Result<()> {
        let line = line.trim();
        if line.is_empty() {
            return Ok(());
        }

        // Parse the incoming message
        let message: McpMessage = serde_json::from_str(line)
            .map_err(|e| anyhow!("Failed to parse MCP message: {}", e))?;

        match message {
            McpMessage::Request(request) => {
                let response = Self::handle_request(request, bridge, capabilities).await;
                let response_json = serde_json::to_string(&McpMessage::Response(response))?;
                stdout
                    .write_all(format!("{}\n", response_json).as_bytes())
                    .await?;
                stdout.flush().await?;
            }
            McpMessage::Response(_) => {
                // Servers don't typically handle responses
                tracing::warn!("Received unexpected response message");
            }
            McpMessage::Notification(_) => {
                // Handle notifications (not implemented yet)
                tracing::debug!("Received notification (not implemented)");
            }
        }

        Ok(())
    }

    /// Handle an MCP request
    async fn handle_request(
        request: Request,
        bridge: &Arc<RwLock<VegaToMcpBridge>>,
        capabilities: &ServerCapabilities,
    ) -> Response {
        match request.method.as_str() {
            "initialize" => Self::handle_initialize(request, capabilities).await,
            "tools/list" => Self::handle_list_tools(request, bridge).await,
            "tools/call" => Self::handle_call_tool(request, bridge).await,
            "notifications/initialized" => Self::handle_initialized(request).await,
            _ => Self::create_error_response(
                request.id,
                ErrorCode::MethodNotFound,
                &format!("Method '{}' not found", request.method),
            ),
        }
    }

    /// Handle initialize request
    async fn handle_initialize(request: Request, capabilities: &ServerCapabilities) -> Response {
        let server_info = serde_json::json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {
                "tools": capabilities.tools.as_ref().map(|t| serde_json::json!({
                    "listChanged": t.list_changed
                })),
                "logging": capabilities.logging.as_ref().map(|l| serde_json::json!({
                    "levels": l.levels
                }))
            },
            "serverInfo": {
                "name": "vega-mcp-server",
                "version": "0.1.0"
            }
        });

        Response {
            id: request.id,
            result: Some(server_info),
            error: None,
        }
    }

    /// Handle initialized notification
    async fn handle_initialized(request: Request) -> Response {
        tracing::info!("MCP client initialized");

        Response {
            id: request.id,
            result: Some(Value::Null),
            error: None,
        }
    }

    /// Handle list tools request
    async fn handle_list_tools(
        request: Request,
        bridge: &Arc<RwLock<VegaToMcpBridge>>,
    ) -> Response {
        match bridge.read().await.get_all_tools() {
            Ok(tools) => {
                let tools_json = serde_json::json!({
                    "tools": tools
                });

                Response {
                    id: request.id,
                    result: Some(tools_json),
                    error: None,
                }
            }
            Err(e) => Self::create_error_response(
                request.id,
                ErrorCode::InternalError,
                &format!("Failed to list tools: {}", e),
            ),
        }
    }

    /// Handle call tool request
    async fn handle_call_tool(request: Request, bridge: &Arc<RwLock<VegaToMcpBridge>>) -> Response {
        let params = match request.params {
            Some(params) => params,
            None => {
                return Self::create_error_response(
                    request.id,
                    ErrorCode::InvalidParams,
                    "Missing parameters for tool call",
                );
            }
        };

        let tool_name = match params.get("name").and_then(|n| n.as_str()) {
            Some(name) => name,
            None => {
                return Self::create_error_response(
                    request.id,
                    ErrorCode::InvalidParams,
                    "Missing 'name' parameter",
                );
            }
        };

        let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);

        match bridge.read().await.call_tool(tool_name, arguments).await {
            Ok(result) => {
                let response_content = serde_json::json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
                    }],
                    "isError": false
                });

                Response {
                    id: request.id,
                    result: Some(response_content),
                    error: None,
                }
            }
            Err(e) => {
                let error_content = serde_json::json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Error calling tool '{}': {}", tool_name, e)
                    }],
                    "isError": true
                });

                Response {
                    id: request.id,
                    result: Some(error_content),
                    error: None,
                }
            }
        }
    }

    /// Create an error response
    fn create_error_response(id: Value, code: ErrorCode, message: &str) -> Response {
        Response {
            id,
            result: None,
            error: Some(McpError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }

    /// Stop the server
    pub async fn stop(self) -> Result<()> {
        *self.running.write().await = false;

        if let Some(handle) = self.task_handle {
            handle.abort();
        }

        tracing::info!("MCP server stopped");
        Ok(())
    }

    /// Get server configuration
    pub fn config(&self) -> &McpServerConfig {
        &self.config
    }

    /// Get server capabilities
    pub fn capabilities(&self) -> &ServerCapabilities {
        &self.capabilities
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::config::McpServerConfig;

    #[tokio::test]
    async fn test_mcp_server_creation() {
        let config = McpServerConfig::default();
        let server = McpServer::new(config).await.unwrap();

        assert!(!server.config.exposed_tools.is_empty());
        assert!(server.capabilities.tools.is_some());
    }

    #[tokio::test]
    async fn test_server_capabilities() {
        let capabilities = ServerCapabilities::default();

        assert!(capabilities.tools.is_some());
        assert!(capabilities.logging.is_some());
        assert!(capabilities.resources.is_none());
        assert!(capabilities.prompts.is_none());
    }

    #[test]
    fn test_error_response_creation() {
        let response = McpServer::create_error_response(
            Value::Number(1.into()),
            ErrorCode::InvalidParams,
            "Test error",
        );

        assert!(response.error.is_some());
        assert!(response.result.is_none());
        assert_eq!(response.error.unwrap().message, "Test error");
    }
}
