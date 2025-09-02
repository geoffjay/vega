//! # MCP Configuration
//!
//! This module defines configuration structures for MCP clients and servers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main configuration for MCP functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// Server configuration (if running as an MCP server)
    pub server: Option<McpServerConfig>,
    /// Client configurations for connecting to external MCP servers
    pub clients: HashMap<String, McpClientConfig>,
    /// Global MCP settings
    pub settings: McpSettings,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            server: None,
            clients: HashMap::new(),
            settings: McpSettings::default(),
        }
    }
}

/// Configuration for an MCP server instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Name of the server
    pub name: String,
    /// Description of the server
    pub description: String,
    /// Version of the server
    pub version: String,
    /// Transport configuration
    pub transport: TransportConfig,
    /// Which tools to expose via MCP
    pub exposed_tools: Vec<String>,
    /// Server-specific settings
    pub settings: ServerSettings,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            name: "vega-mcp-server".to_string(),
            description: "Vega AI Agent MCP Server".to_string(),
            version: "0.1.0".to_string(),
            transport: TransportConfig::default(),
            exposed_tools: vec![
                "bash".to_string(),
                "read_file".to_string(),
                "edit_file".to_string(),
                "list_files".to_string(),
                "code_search".to_string(),
                "web_search".to_string(),
                "read_logs".to_string(),
            ],
            settings: ServerSettings::default(),
        }
    }
}

/// Configuration for an MCP client connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpClientConfig {
    /// Name of the server to connect to
    pub server_name: String,
    /// Command to start the MCP server
    pub command: String,
    /// Arguments for the server command
    pub args: Vec<String>,
    /// Environment variables for the server process
    pub env: HashMap<String, String>,
    /// Working directory for the server process
    pub cwd: Option<String>,
    /// Transport configuration
    pub transport: TransportConfig,
    /// Client-specific settings
    pub settings: ClientSettings,
}

impl Default for McpClientConfig {
    fn default() -> Self {
        Self {
            server_name: "external-server".to_string(),
            command: "python".to_string(),
            args: vec!["server.py".to_string()],
            env: HashMap::new(),
            cwd: None,
            transport: TransportConfig::default(),
            settings: ClientSettings::default(),
        }
    }
}

/// Transport layer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    /// Transport type (stdio, sse, etc.)
    pub transport_type: TransportType,
    /// Transport-specific options
    pub options: TransportOptions,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            transport_type: TransportType::Stdio,
            options: TransportOptions::default(),
        }
    }
}

/// Available transport types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransportType {
    /// Standard input/output transport
    Stdio,
    /// Server-Sent Events transport
    Sse,
    /// HTTP transport (future)
    Http,
}

/// Transport-specific configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportOptions {
    /// Timeout for operations (in seconds)
    pub timeout: Option<u64>,
    /// Buffer size for I/O operations
    pub buffer_size: Option<usize>,
    /// Keep-alive settings
    pub keep_alive: Option<bool>,
    /// Additional transport-specific settings
    pub extra: HashMap<String, serde_json::Value>,
}

impl Default for TransportOptions {
    fn default() -> Self {
        Self {
            timeout: Some(30),
            buffer_size: Some(8192),
            keep_alive: Some(true),
            extra: HashMap::new(),
        }
    }
}

/// Global MCP settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSettings {
    /// Enable debug logging for MCP operations
    pub debug: bool,
    /// Maximum number of concurrent tool calls
    pub max_concurrent_calls: usize,
    /// Default timeout for tool calls (in seconds)
    pub default_timeout: u64,
    /// Enable automatic reconnection on connection loss
    pub auto_reconnect: bool,
    /// Retry attempts for failed operations
    pub retry_attempts: usize,
}

impl Default for McpSettings {
    fn default() -> Self {
        Self {
            debug: false,
            max_concurrent_calls: 10,
            default_timeout: 30,
            auto_reconnect: true,
            retry_attempts: 3,
        }
    }
}

/// Server-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSettings {
    /// Enable CORS for HTTP transport
    pub enable_cors: bool,
    /// Maximum request size (in bytes)
    pub max_request_size: usize,
    /// Rate limiting settings
    pub rate_limit: Option<RateLimit>,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            enable_cors: true,
            max_request_size: 10 * 1024 * 1024, // 10MB
            rate_limit: None,
        }
    }
}

/// Client-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientSettings {
    /// Enable connection pooling
    pub connection_pooling: bool,
    /// Maximum number of connections to maintain
    pub max_connections: usize,
    /// Connection timeout (in seconds)
    pub connection_timeout: u64,
}

impl Default for ClientSettings {
    fn default() -> Self {
        Self {
            connection_pooling: false,
            max_connections: 1,
            connection_timeout: 10,
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    /// Maximum requests per window
    pub max_requests: usize,
    /// Time window in seconds
    pub window_seconds: u64,
}

/// Information about an MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    /// Server name
    pub name: String,
    /// Server version
    pub version: String,
    /// Server capabilities
    pub capabilities: ServerCapabilities,
}

/// Server capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    /// Supports listing tools
    pub tools: Option<ToolsCapability>,
    /// Supports listing resources
    pub resources: Option<ResourcesCapability>,
    /// Supports listing prompts
    pub prompts: Option<PromptsCapability>,
    /// Supports logging
    pub logging: Option<LoggingCapability>,
}

/// Tools capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
    /// Supports listing available tools
    pub list_changed: Option<bool>,
}

/// Resources capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesCapability {
    /// Supports subscribing to resource changes
    pub subscribe: Option<bool>,
    /// Supports listing available resources
    pub list_changed: Option<bool>,
}

/// Prompts capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsCapability {
    /// Supports listing available prompts
    pub list_changed: Option<bool>,
}

/// Logging capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingCapability {
    /// Supported log levels
    pub levels: Option<Vec<String>>,
}

impl McpConfig {
    /// Load MCP configuration from a file
    pub fn from_file(path: &str) -> Result<Self, anyhow::Error> {
        let content = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save MCP configuration to a file
    pub fn to_file(&self, path: &str) -> Result<(), anyhow::Error> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Add a client configuration
    pub fn add_client(&mut self, name: String, config: McpClientConfig) {
        self.clients.insert(name, config);
    }

    /// Remove a client configuration
    pub fn remove_client(&mut self, name: &str) {
        self.clients.remove(name);
    }

    /// Enable the MCP server with the given configuration
    pub fn enable_server(&mut self, config: McpServerConfig) {
        self.server = Some(config);
    }

    /// Disable the MCP server
    pub fn disable_server(&mut self) {
        self.server = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_config_default() {
        let config = McpConfig::default();
        assert!(config.server.is_none());
        assert!(config.clients.is_empty());
        assert!(!config.settings.debug);
    }

    #[test]
    fn test_mcp_server_config_default() {
        let config = McpServerConfig::default();
        assert_eq!(config.name, "vega-mcp-server");
        assert!(!config.exposed_tools.is_empty());
        assert!(config.exposed_tools.contains(&"bash".to_string()));
    }

    #[test]
    fn test_mcp_client_config_default() {
        let config = McpClientConfig::default();
        assert_eq!(config.server_name, "external-server");
        assert_eq!(config.command, "python");
        assert!(!config.args.is_empty());
    }

    #[test]
    fn test_transport_config() {
        let config = TransportConfig::default();
        assert!(matches!(config.transport_type, TransportType::Stdio));
        assert_eq!(config.options.timeout, Some(30));
    }

    #[test]
    fn test_mcp_config_client_management() {
        let mut config = McpConfig::default();
        let client_config = McpClientConfig::default();

        config.add_client("test".to_string(), client_config);
        assert!(config.clients.contains_key("test"));

        config.remove_client("test");
        assert!(!config.clients.contains_key("test"));
    }

    #[test]
    fn test_mcp_config_server_management() {
        let mut config = McpConfig::default();
        assert!(config.server.is_none());

        let server_config = McpServerConfig::default();
        config.enable_server(server_config);
        assert!(config.server.is_some());

        config.disable_server();
        assert!(config.server.is_none());
    }
}
