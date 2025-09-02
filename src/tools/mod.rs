//! # Tool System for Vega AI Agent
//!
//! This module provides a comprehensive set of tools that enable the Vega AI agent
//! to interact with the system, manipulate files, search code, browse the web, and more.
//! All tools are implemented using the Rig framework's tool system and include
//! safety checks and validation.
//!
//! ## Available Tools
//!
//! - [`BashTool`] - Execute shell commands with safety checks
//! - [`CodeSearchTool`] - Search through code using ripgrep
//! - [`WebSearchTool`] - Perform web searches using DuckDuckGo
//! - [`ReadFileTool`] - Read file contents with encoding detection
//! - [`EditFileTool`] - Create and edit files with backup support
//! - [`ListFilesTool`] - List directory contents with filtering
//! - [`ReadLogsTool`] - Read and filter log entries
//!
//! ## Confirmed Tools
//!
//! For potentially destructive operations, confirmed versions are available:
//! - [`ConfirmedBashTool`] - Bash tool with user confirmation
//! - [`ConfirmedEditFileTool`] - Edit tool with user confirmation
//!
//! ## Safety Features
//!
//! All tools include comprehensive safety measures:
//! - Input validation and sanitization
//! - Path traversal protection
//! - Command injection prevention
//! - Resource usage limits
//! - User confirmation for destructive operations
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use vega::tools::{BashTool, ReadFileTool, RigTool};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create tools
//!     let bash_tool = BashTool::new();
//!     let read_tool = ReadFileTool::new();
//!     
//!     // Use tools (would typically be called by the agent)
//!     // let result = bash_tool.call(args).await?;
//!     
//!     Ok(())
//! }
//! ```

// Re-export the rig Tool trait for convenience
pub use rig::tool::Tool as RigTool;

// Tool modules
pub mod bash;
pub mod code_search;
pub mod confirmed;
pub mod edit_file;
pub mod list_files;
pub mod read_file;
pub mod read_logs;
pub mod web_search;

// Re-export all tools
pub use bash::BashTool;
pub use code_search::CodeSearchTool;
pub use confirmed::{ConfirmedBashTool, ConfirmedEditFileTool};
pub use edit_file::EditFileTool;
pub use list_files::ListFilesTool;
pub use read_file::ReadFileTool;
pub use read_logs::ReadLogsTool;
pub use web_search::WebSearchTool;

/// Common error types for all tools in the system.
///
/// This enum provides a unified error handling system across all tools,
/// making it easier to handle and report errors consistently.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    /// I/O operation failed (file system, network, etc.)
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// HTTP request failed (network issues, server errors, etc.)
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    /// JSON parsing or serialization failed
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// Command execution failed with details
    #[error("Command execution failed: {0}")]
    Command(String),
    /// Requested file or directory does not exist
    #[error("File not found: {0}")]
    FileNotFound(String),
    /// Operation denied due to insufficient permissions
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    /// Input validation failed or invalid parameters provided
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// Creates a collection of all available tools for use by agents.
///
/// This function instantiates all available tools and returns them in a format
/// that can be used by the Rig framework's agent system. The tools are returned
/// as trait objects to allow for dynamic dispatch.
///
/// # Returns
///
/// A vector of boxed tool instances that implement the necessary traits for
/// agent integration.
///
/// # Example
///
/// ```rust,no_run
/// use vega::tools::create_all_tools;
///
/// let tools = create_all_tools();
/// println!("Created {} tools", tools.len());
/// ```
pub fn create_all_tools() -> Vec<Box<dyn std::any::Any + Send + Sync>> {
    vec![
        Box::new(WebSearchTool::new()),
        Box::new(BashTool::new()),
        Box::new(CodeSearchTool::new()),
        Box::new(ReadFileTool::new()),
        Box::new(EditFileTool::new()),
        Box::new(ListFilesTool::new()),
        Box::new(ReadLogsTool::new()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_all_tools() {
        let tools = create_all_tools();
        assert_eq!(tools.len(), 7);
    }
}
