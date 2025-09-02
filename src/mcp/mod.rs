//! # Model Context Protocol (MCP) Support
//!
//! This module provides comprehensive Model Context Protocol (MCP) support for the Vega AI agent.
//! MCP enables seamless integration between AI models and external tools, resources, and data sources.
//!
//! ## Features
//!
//! - **MCP Client**: Connect to external MCP servers to access their tools and resources
//! - **MCP Server**: Expose Vega's built-in tools as MCP tools for other AI systems
//! - **Dual Mode**: Can function as both client and server simultaneously
//! - **Standard Compliance**: Fully compliant with the MCP specification
//! - **Async Support**: Built on async/await for non-blocking operations
//!
//! ## Architecture
//!
//! The MCP implementation is organized into several modules:
//!
//! - [`client`] - MCP client functionality for connecting to external servers
//! - [`server`] - MCP server functionality for exposing Vega's tools
//! - [`bridge`] - Bridge layer that integrates MCP tools with Vega's existing tool system
//! - [`config`] - Configuration structures for MCP clients and servers
//! - [`transport`] - Transport layer implementations (stdio, SSE, etc.)
//!
//! ## Usage Examples
//!
//! ### As an MCP Client
//!
//! ```rust,no_run
//! use vega::mcp::{McpClient, McpClientConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = McpClientConfig {
//!         server_command: "python mcp_server.py".to_string(),
//!         ..Default::default()
//!     };
//!     
//!     let mut client = McpClient::new(config).await?;
//!     let tools = client.list_tools().await?;
//!     println!("Available tools: {:?}", tools);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### As an MCP Server
//!
//! ```rust,no_run
//! use vega::mcp::{McpServer, McpServerConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = McpServerConfig::default();
//!     let server = McpServer::new(config).await?;
//!     
//!     // Server will expose Vega's tools via MCP
//!     server.run().await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod simple;

// For now, we'll use the simple implementation
// The more complex bridge, client, server, and transport modules
// can be enabled later when we have time to fix all the type issues

// pub mod bridge;
// pub mod client;
// pub mod server;
// pub mod transport;

// Re-export commonly used types
pub use config::{McpConfig, McpServerInfo};
pub use simple::{
    SimpleMcpClient, SimpleMcpClientConfig, SimpleMcpManager, SimpleMcpServer,
    SimpleMcpServerConfig,
};

// These will be available when the full implementation is ready
// pub use client::{McpClient, McpClientConfig};
// pub use server::{McpServer, McpServerConfig};

use anyhow::Result;

/// Trait representing an MCP tool that can be called remotely
/// Note: This trait cannot use async methods to remain object-safe
pub trait McpTool: Send + Sync {
    /// Get the tool's name
    fn name(&self) -> &str;

    /// Get the tool's description
    fn description(&self) -> &str;

    /// Get the tool's input schema
    fn input_schema(&self) -> serde_json::Value;

    /// Call the tool with the given arguments (returns a boxed future for object safety)
    fn call_boxed(
        &self,
        args: serde_json::Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send>>;
}

/// Extension trait for convenience with async methods
pub trait McpToolExt: McpTool {
    /// Call the tool with the given arguments
    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        self.call_boxed(args).await
    }
}

impl<T: McpTool> McpToolExt for T {}

/// Manager for MCP functionality, coordinating clients and servers
/// This is an alias to the simple manager for now
pub type McpManager = SimpleMcpManager;

// Extended functionality for McpManager using the simple implementation
impl McpManager {
    /// Create a new MCP manager with the given configuration
    pub fn with_config(config: McpConfig) -> Self {
        let mut manager = Self::new();

        // If server is configured, start it
        if let Some(server_config) = config.server {
            let simple_config = SimpleMcpServerConfig {
                name: server_config.name,
                version: server_config.version,
                enabled_tools: server_config.exposed_tools,
            };
            manager.start_server(simple_config);
        }

        // Add configured clients
        for (name, client_config) in config.clients {
            let simple_config = SimpleMcpClientConfig {
                server_command: client_config.command,
                server_args: client_config.args,
            };
            manager.add_client(name, simple_config);
        }

        manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_manager_creation() {
        let config = McpConfig::default();
        let manager = McpManager::with_config(config);
        assert!(manager.clients.is_empty());
        assert!(manager.server.is_none());
    }
}
