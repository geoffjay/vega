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

/// Common error types for tools
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Command execution failed: {0}")]
    Command(String),
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// Helper function to create a tool collection for agents
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
