//! # MCP Client Implementation
//!
//! This module provides MCP client functionality for connecting to external MCP servers
//! and accessing their tools and resources.

use anyhow::{Result, anyhow};
use rust_mcp_schema::{McpMessage, Request, Response, Tool as McpToolDef};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, timeout};

use super::McpTool;
use super::bridge::VegaMcpTool;
use super::config::{McpClientConfig, TransportConfig};
use super::transport::{McpTransport, MessageRouter, RequestBuilder, TransportFactory};

/// Configuration for MCP client
pub use super::config::McpClientConfig;

/// MCP client for connecting to external servers
#[derive(Debug)]
pub struct McpClient {
    /// Client configuration
    config: McpClientConfig,
    /// Transport layer for communication
    transport: Option<Box<dyn McpTransport>>,
    /// Message router for request/response correlation
    router: Arc<RwLock<MessageRouter>>,
    /// Available tools from the server
    tools: Arc<RwLock<HashMap<String, McpToolDef>>>,
    /// Available resources from the server
    resources: Arc<RwLock<HashMap<String, Value>>>,
    /// Connection state
    connected: bool,
    /// Server information
    server_info: Option<Value>,
}

impl McpClient {
    /// Create a new MCP client with the given configuration
    pub async fn new(config: McpClientConfig) -> Result<Self> {
        let transport = TransportFactory::create(config.transport.clone())?;

        let mut client = Self {
            config,
            transport: Some(transport),
            router: Arc::new(RwLock::new(MessageRouter::new())),
            tools: Arc::new(RwLock::new(HashMap::new())),
            resources: Arc::new(RwLock::new(HashMap::new())),
            connected: false,
            server_info: None,
        };

        // Establish connection and initialize
        client.connect().await?;
        client.initialize().await?;

        Ok(client)
    }

    /// Connect to the MCP server
    async fn connect(&mut self) -> Result<()> {
        if let Some(transport) = &mut self.transport {
            // For stdio transport, we need to start the process
            if let Some(stdio_transport) = transport
                .as_any()
                .downcast_mut::<crate::mcp::transport::StdioTransport>()
            {
                stdio_transport
                    .connect(&self.config.command, &self.config.args)
                    .await?;
            }
            self.connected = true;
        }
        Ok(())
    }

    /// Initialize the MCP session
    async fn initialize(&mut self) -> Result<()> {
        let mut router = self.router.write().await;
        let id = router.next_id();
        let rx = router.register_request(id);
        drop(router);

        let client_info = serde_json::json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {
                "tools": {
                    "list_changed": true
                },
                "resources": {
                    "list_changed": true,
                    "subscribe": false
                }
            },
            "clientInfo": {
                "name": "vega-mcp-client",
                "version": "0.1.0"
            }
        });

        let request = RequestBuilder::initialize(id, client_info);
        self.send_request(request).await?;

        // Wait for initialize response
        let response = timeout(Duration::from_secs(10), rx)
            .await
            .map_err(|_| anyhow!("Timeout waiting for initialize response"))?
            .map_err(|_| anyhow!("Initialize request cancelled"))?;

        if let Some(result) = response.result {
            self.server_info = Some(result);
            tracing::info!("MCP client initialized successfully");
        } else if let Some(error) = response.error {
            return Err(anyhow!("Initialize failed: {:?}", error));
        }

        // Load available tools and resources
        self.refresh_tools().await?;
        self.refresh_resources().await?;

        Ok(())
    }

    /// Send a request to the server
    async fn send_request(&mut self, request: Request) -> Result<()> {
        if let Some(transport) = &mut self.transport {
            let message = McpMessage::Request(request);
            transport.send(message).await?;
        } else {
            return Err(anyhow!("No transport available"));
        }
        Ok(())
    }

    /// Refresh the list of available tools
    async fn refresh_tools(&mut self) -> Result<()> {
        let mut router = self.router.write().await;
        let id = router.next_id();
        let rx = router.register_request(id);
        drop(router);

        let request = RequestBuilder::list_tools(id);
        self.send_request(request).await?;

        let response = timeout(Duration::from_secs(10), rx)
            .await
            .map_err(|_| anyhow!("Timeout waiting for tools list"))?
            .map_err(|_| anyhow!("Tools list request cancelled"))?;

        if let Some(result) = response.result {
            if let Some(tools_array) = result.get("tools").and_then(|t| t.as_array()) {
                let mut tools = self.tools.write().await;
                tools.clear();

                for tool_value in tools_array {
                    if let Ok(tool) = serde_json::from_value::<McpToolDef>(tool_value.clone()) {
                        tools.insert(tool.name.clone(), tool);
                    }
                }

                tracing::info!("Loaded {} tools from MCP server", tools.len());
            }
        } else if let Some(error) = response.error {
            tracing::warn!("Failed to list tools: {:?}", error);
        }

        Ok(())
    }

    /// Refresh the list of available resources
    async fn refresh_resources(&mut self) -> Result<()> {
        let mut router = self.router.write().await;
        let id = router.next_id();
        let rx = router.register_request(id);
        drop(router);

        let request = RequestBuilder::list_resources(id);
        self.send_request(request).await?;

        let response = timeout(Duration::from_secs(10), rx)
            .await
            .map_err(|_| anyhow!("Timeout waiting for resources list"))?
            .map_err(|_| anyhow!("Resources list request cancelled"))?;

        if let Some(result) = response.result {
            if let Some(resources_array) = result.get("resources").and_then(|r| r.as_array()) {
                let mut resources = self.resources.write().await;
                resources.clear();

                for resource_value in resources_array {
                    if let Some(uri) = resource_value.get("uri").and_then(|u| u.as_str()) {
                        resources.insert(uri.to_string(), resource_value.clone());
                    }
                }

                tracing::info!("Loaded {} resources from MCP server", resources.len());
            }
        } else if let Some(error) = response.error {
            tracing::warn!("Failed to list resources: {:?}", error);
        }

        Ok(())
    }

    /// Get all available tools as Vega-compatible tools
    pub async fn get_tools(&self) -> Result<Vec<Box<dyn McpTool>>> {
        let tools = self.tools.read().await;
        let mut vega_tools: Vec<Box<dyn McpTool>> = Vec::new();

        for (name, tool_def) in tools.iter() {
            let vega_tool = VegaMcpTool::new(name.clone(), tool_def.clone(), self.router.clone());
            vega_tools.push(Box::new(vega_tool));
        }

        Ok(vega_tools)
    }

    /// Call a tool on the remote server
    pub async fn call_tool(&mut self, name: &str, arguments: Option<Value>) -> Result<Value> {
        let mut router = self.router.write().await;
        let id = router.next_id();
        let rx = router.register_request(id);
        drop(router);

        let request = RequestBuilder::call_tool(id, name, arguments);
        self.send_request(request).await?;

        let response = timeout(Duration::from_secs(30), rx)
            .await
            .map_err(|_| anyhow!("Timeout waiting for tool call response"))?
            .map_err(|_| anyhow!("Tool call request cancelled"))?;

        if let Some(result) = response.result {
            Ok(result)
        } else if let Some(error) = response.error {
            Err(anyhow!("Tool call failed: {:?}", error))
        } else {
            Err(anyhow!("No result or error in response"))
        }
    }

    /// List available tools
    pub async fn list_tools(&self) -> Result<Vec<String>> {
        let tools = self.tools.read().await;
        Ok(tools.keys().cloned().collect())
    }

    /// Get tool definition
    pub async fn get_tool_definition(&self, name: &str) -> Result<McpToolDef> {
        let tools = self.tools.read().await;
        tools
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow!("Tool '{}' not found", name))
    }

    /// List available resources
    pub async fn list_resources(&self) -> Result<Vec<String>> {
        let resources = self.resources.read().await;
        Ok(resources.keys().cloned().collect())
    }

    /// Read a resource from the server
    pub async fn read_resource(&mut self, uri: &str) -> Result<Value> {
        let mut router = self.router.write().await;
        let id = router.next_id();
        let rx = router.register_request(id);
        drop(router);

        let request = RequestBuilder::read_resource(id, uri);
        self.send_request(request).await?;

        let response = timeout(Duration::from_secs(30), rx)
            .await
            .map_err(|_| anyhow!("Timeout waiting for resource read response"))?
            .map_err(|_| anyhow!("Resource read request cancelled"))?;

        if let Some(result) = response.result {
            Ok(result)
        } else if let Some(error) = response.error {
            Err(anyhow!("Resource read failed: {:?}", error))
        } else {
            Err(anyhow!("No result or error in response"))
        }
    }

    /// Get server information
    pub fn get_server_info(&self) -> Option<&Value> {
        self.server_info.as_ref()
    }

    /// Check if the client is connected
    pub fn is_connected(&self) -> bool {
        self.connected && self.transport.as_ref().map_or(false, |t| t.is_connected())
    }

    /// Disconnect from the server
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut transport) = self.transport.take() {
            transport.close().await?;
        }
        self.connected = false;
        tracing::info!("Disconnected from MCP server");
        Ok(())
    }
}

// Helper trait to enable downcasting for transport
trait AsAny {
    fn as_any(&mut self) -> &mut dyn std::any::Any;
}

impl<T: 'static> AsAny for T {
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// Extend McpTransport with AsAny
impl dyn McpTransport {
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self as &mut dyn std::any::Any
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::config::{TransportConfig, TransportType};

    #[test]
    fn test_mcp_client_config() {
        let config = McpClientConfig {
            server_name: "test-server".to_string(),
            command: "python".to_string(),
            args: vec!["server.py".to_string()],
            env: HashMap::new(),
            cwd: None,
            transport: TransportConfig {
                transport_type: TransportType::Stdio,
                ..Default::default()
            },
            settings: Default::default(),
        };

        assert_eq!(config.server_name, "test-server");
        assert_eq!(config.command, "python");
        assert!(!config.args.is_empty());
    }

    #[tokio::test]
    async fn test_client_tools_storage() {
        let client_config = McpClientConfig::default();

        // Create a mock client (this won't actually connect in tests)
        let router = Arc::new(RwLock::new(MessageRouter::new()));
        let tools = Arc::new(RwLock::new(HashMap::new()));

        // Simulate adding a tool
        let mut tools_map = tools.write().await;
        let mock_tool = McpToolDef {
            name: "test_tool".to_string(),
            description: Some("A test tool".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string"}
                }
            }),
        };
        tools_map.insert("test_tool".to_string(), mock_tool);
        drop(tools_map);

        // Verify tool was stored
        let tools_map = tools.read().await;
        assert!(tools_map.contains_key("test_tool"));
        assert_eq!(tools_map.get("test_tool").unwrap().name, "test_tool");
    }
}
