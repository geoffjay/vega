//! # MCP Bridge
//!
//! This module provides a bridge between MCP tools and Vega's existing tool system,
//! allowing MCP tools to be used seamlessly within Vega's agent framework.

use anyhow::{Result, anyhow};
use rig::tool::Tool as RigTool;
use rust_mcp_schema::Tool as McpToolDef;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::McpTool;
use super::transport::{MessageRouter, RequestBuilder};
use crate::tools::ToolError;

/// A wrapper that makes MCP tools compatible with Vega's tool system
#[derive(Debug)]
pub struct VegaMcpTool {
    /// Tool name
    name: String,
    /// Tool definition from MCP server
    definition: McpToolDef,
    /// Message router for sending requests
    router: Arc<RwLock<MessageRouter>>,
}

impl VegaMcpTool {
    /// Create a new Vega-compatible MCP tool
    pub fn new(name: String, definition: McpToolDef, router: Arc<RwLock<MessageRouter>>) -> Self {
        Self {
            name,
            definition,
            router,
        }
    }
}

impl McpTool for VegaMcpTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        self.definition
            .description
            .as_deref()
            .unwrap_or("No description available")
    }

    fn input_schema(&self) -> Value {
        // Convert the schema to a serde_json::Value
        serde_json::to_value(&self.definition.input_schema).unwrap_or(Value::Null)
    }

    fn call_boxed(
        &self,
        args: Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send>> {
        let router = self.router.clone();
        let name = self.name.clone();

        Box::pin(async move {
            let mut router = router.write().await;
            let id = router.next_id();
            let rx = router.register_request(id);
            drop(router);

            let request = RequestBuilder::call_tool(id, &name, Some(args));

            // Note: In a real implementation, we would need access to the transport
            // For now, this is a placeholder that shows the structure
            // The actual implementation would need to be coordinated with the client

            Err(anyhow!(
                "Tool call not implemented in bridge - needs client transport"
            ))
        })
    }
}

/// Bridge that converts Vega's native tools to MCP format
#[derive(Debug)]
pub struct VegaToMcpBridge {
    /// Map of tool names to their configurations for runtime instantiation
    tool_configs: HashMap<String, VegaToolConfig>,
}

/// Configuration for a Vega tool that can be instantiated when needed
#[derive(Debug, Clone)]
pub enum VegaToolConfig {
    Bash,
    ReadFile,
    EditFile,
    ListFiles,
    CodeSearch,
    WebSearch,
    ReadLogs,
}

impl VegaToMcpBridge {
    /// Create a new bridge with Vega tools
    pub fn new() -> Self {
        Self {
            tool_configs: HashMap::new(),
        }
    }

    /// Add a Vega tool to be exposed via MCP
    pub fn add_tool(&mut self, name: String, config: VegaToolConfig) {
        self.tool_configs.insert(name, config);
    }

    /// Convert a Vega tool to MCP tool definition
    pub fn to_mcp_tool_definition(&self, name: &str) -> Result<McpToolDef> {
        let config = self
            .tool_configs
            .get(name)
            .ok_or_else(|| anyhow!("Tool '{}' not found", name))?;

        self.config_to_mcp_definition(name, config)
    }

    /// Convert a tool config to MCP definition
    fn config_to_mcp_definition(&self, name: &str, config: &VegaToolConfig) -> Result<McpToolDef> {
        // Use the predefined tool definitions from McpToolFactory
        let definitions = McpToolFactory::create_mcp_tools()?;

        definitions
            .into_iter()
            .find(|def| def.name == name)
            .ok_or_else(|| anyhow!("No MCP definition found for tool '{}'", name))
    }

    /// Get all available tools as MCP definitions
    pub fn get_all_tools(&self) -> Result<Vec<McpToolDef>> {
        let mut tools = Vec::new();

        for (name, config) in &self.tool_configs {
            let definition = self.config_to_mcp_definition(name, config)?;
            tools.push(definition);
        }

        Ok(tools)
    }

    /// Call a Vega tool through the bridge
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value> {
        let config = self
            .tool_configs
            .get(name)
            .ok_or_else(|| anyhow!("Tool '{}' not found", name))?;

        self.call_tool_by_config(config, arguments).await
    }

    /// Call a tool based on its configuration
    async fn call_tool_by_config(
        &self,
        config: &VegaToolConfig,
        arguments: Value,
    ) -> Result<Value> {
        use crate::tools::*;

        // Convert the arguments to the format expected by Rig tools
        let args_str = serde_json::to_string(&arguments)?;

        let result = match config {
            VegaToolConfig::Bash => {
                let tool = BashTool::new();
                tool.call(&args_str).await
            }
            VegaToolConfig::ReadFile => {
                let tool = ReadFileTool::new();
                tool.call(&args_str).await
            }
            VegaToolConfig::EditFile => {
                let tool = EditFileTool::new();
                tool.call(&args_str).await
            }
            VegaToolConfig::ListFiles => {
                let tool = ListFilesTool::new();
                tool.call(&args_str).await
            }
            VegaToolConfig::CodeSearch => {
                let tool = CodeSearchTool::new();
                tool.call(&args_str).await
            }
            VegaToolConfig::WebSearch => {
                let tool = WebSearchTool::new();
                tool.call(&args_str).await
            }
            VegaToolConfig::ReadLogs => {
                let tool = ReadLogsTool::new();
                tool.call(&args_str).await
            }
        }
        .map_err(|e| anyhow!("Tool call failed: {}", e))?;

        // Parse the result back to JSON
        let result_value: Value =
            serde_json::from_str(&result).unwrap_or_else(|_| Value::String(result));

        Ok(result_value)
    }

    /// List all available tool names
    pub fn list_tools(&self) -> Vec<String> {
        self.tool_configs.keys().cloned().collect()
    }
}

/// MCP tool call request structure
#[derive(Debug, Serialize, Deserialize)]
pub struct McpToolCallRequest {
    /// Tool name to call
    pub name: String,
    /// Arguments for the tool
    pub arguments: Option<Value>,
}

/// MCP tool call response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct McpToolCallResponse {
    /// Content returned by the tool
    pub content: Vec<McpContent>,
    /// Whether the tool call was an error
    pub is_error: Option<bool>,
}

/// Content structure for MCP responses
#[derive(Debug, Serialize, Deserialize)]
pub struct McpContent {
    /// Content type (text, image, etc.)
    #[serde(rename = "type")]
    pub content_type: String,
    /// The actual content
    pub text: Option<String>,
}

/// Factory for creating MCP-compatible tools from Vega's tool system
pub struct McpToolFactory;

impl McpToolFactory {
    /// Create MCP tool definitions from Vega's available tools
    pub fn create_mcp_tools() -> Result<Vec<McpToolDef>> {
        let mut tools = Vec::new();

        // Bash Tool
        tools.push(McpToolDef {
            name: "bash".to_string(),
            description: Some(
                "Execute shell commands with safety checks and timeout protection".to_string(),
            ),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute"
                    },
                    "timeout_seconds": {
                        "type": "number",
                        "description": "Timeout in seconds (default: 30)",
                        "default": 30
                    },
                    "working_directory": {
                        "type": "string",
                        "description": "Working directory for the command"
                    }
                },
                "required": ["command"]
            }),
        });

        // Read File Tool
        tools.push(McpToolDef {
            name: "read_file".to_string(),
            description: Some(
                "Read file contents with safety checks and encoding detection".to_string(),
            ),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read"
                    },
                    "encoding": {
                        "type": "string",
                        "description": "Text encoding to use (auto-detected if not specified)"
                    },
                    "max_size_mb": {
                        "type": "number",
                        "description": "Maximum file size in MB (default: 10)",
                        "default": 10
                    },
                    "line_range": {
                        "type": "array",
                        "description": "Line range [start_line, end_line] (1-indexed)",
                        "items": {"type": "number"},
                        "minItems": 2,
                        "maxItems": 2
                    }
                },
                "required": ["path"]
            }),
        });

        // Edit File Tool
        tools.push(McpToolDef {
            name: "edit_file".to_string(),
            description: Some(
                "Create or edit files with backup support and safety validation".to_string(),
            ),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to edit or create"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file"
                    },
                    "create_if_missing": {
                        "type": "boolean",
                        "description": "Create file if it doesn't exist (default: false)",
                        "default": false
                    },
                    "backup": {
                        "type": "boolean",
                        "description": "Create backup of existing file (default: false)",
                        "default": false
                    },
                    "encoding": {
                        "type": "string",
                        "description": "Text encoding to use (default: UTF-8)",
                        "default": "utf-8"
                    }
                },
                "required": ["path", "content"]
            }),
        });

        // List Files Tool
        tools.push(McpToolDef {
            name: "list_files".to_string(),
            description: Some(
                "List directory contents with filtering and metadata options".to_string(),
            ),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "directory": {
                        "type": "string",
                        "description": "Directory path to list files from"
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "List files recursively (default: false)",
                        "default": false
                    },
                    "include_hidden": {
                        "type": "boolean",
                        "description": "Include hidden files (default: false)",
                        "default": false
                    },
                    "file_types": {
                        "type": "array",
                        "description": "File extensions to filter by",
                        "items": {"type": "string"}
                    },
                    "max_files": {
                        "type": "number",
                        "description": "Maximum number of files to return (default: 1000)",
                        "default": 1000
                    }
                },
                "required": ["directory"]
            }),
        });

        // Code Search Tool
        tools.push(McpToolDef {
            name: "code_search".to_string(),
            description: Some(
                "Search through code using ripgrep with advanced pattern matching".to_string(),
            ),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The regex pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "File or directory path to search in"
                    },
                    "case_sensitive": {
                        "type": "boolean",
                        "description": "Whether search should be case sensitive (default: false)",
                        "default": false
                    },
                    "whole_word": {
                        "type": "boolean",
                        "description": "Whether to match whole words only (default: false)",
                        "default": false
                    },
                    "file_type": {
                        "type": "string",
                        "description": "File type filter (e.g., 'rust', 'js', 'py')"
                    },
                    "max_results": {
                        "type": "number",
                        "description": "Maximum number of results (default: 50)",
                        "default": 50
                    }
                },
                "required": ["pattern", "path"]
            }),
        });

        // Web Search Tool
        tools.push(McpToolDef {
            name: "web_search".to_string(),
            description: Some(
                "Perform web searches using DuckDuckGo to find current information".to_string(),
            ),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query string"
                    },
                    "max_results": {
                        "type": "number",
                        "description": "Maximum number of results (default: 5)",
                        "default": 5
                    }
                },
                "required": ["query"]
            }),
        });

        Ok(tools)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_tool_factory() {
        let tools = McpToolFactory::create_mcp_tools().unwrap();
        assert!(!tools.is_empty());

        let bash_tool = tools.iter().find(|t| t.name == "bash").unwrap();
        assert!(bash_tool.description.is_some());
        assert!(bash_tool.input_schema.is_object());
    }

    #[test]
    fn test_vega_to_mcp_bridge() {
        let bridge = VegaToMcpBridge::new();
        assert!(bridge.list_tools().is_empty());
    }

    #[test]
    fn test_mcp_content_serialization() {
        let content = McpContent {
            content_type: "text".to_string(),
            text: Some("Hello, world!".to_string()),
        };

        let serialized = serde_json::to_string(&content).unwrap();
        assert!(serialized.contains("text"));
        assert!(serialized.contains("Hello, world!"));
    }
}
