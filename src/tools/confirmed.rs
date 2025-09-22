use anyhow::Result;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use tracing::trace;

use std::io::{self, Write};

use super::{
    BashTool, EditFileTool, ToolError,
    bash::{BashArgs, BashOutput},
    edit_file::{EditFileArgs, EditFileOutput},
};

/// Wrapper for tools that require user confirmation
pub struct ConfirmedTool<T> {
    inner: T,
    yolo: bool,
}

impl<T> ConfirmedTool<T> {
    pub fn new(inner: T, yolo: bool) -> Self {
        Self { inner, yolo }
    }

    /// Prompt user for confirmation
    fn confirm_execution(&self, tool_name: &str, description: &str) -> Result<bool, ToolError> {
        trace!("Tool execution requested: {} - {}", tool_name, description);

        if self.yolo {
            trace!("YOLO mode enabled, auto-confirming tool execution");
            return Ok(true);
        }

        // Pause any streaming progress indicators to avoid interference
        crate::streaming::pause_progress();

        println!("\n🔧 Tool Execution Request:");
        println!("Tool: {}", tool_name);
        println!("Action: {}", description);
        print!("Do you want to proceed? (y/N): ");
        io::stdout().flush().map_err(|e| ToolError::Io(e))?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| ToolError::Io(e))?;

        let response = input.trim().to_lowercase();
        let confirmed = response == "y" || response == "yes";

        trace!(
            "User response to tool confirmation: '{}' -> {}",
            response, confirmed
        );

        // Resume streaming progress indicators after user interaction
        crate::streaming::resume_progress();

        Ok(confirmed)
    }
}

/// Confirmed Bash Tool
pub struct ConfirmedBashTool {
    inner: ConfirmedTool<BashTool>,
}

impl ConfirmedBashTool {
    pub fn new(yolo: bool) -> Self {
        Self {
            inner: ConfirmedTool::new(BashTool::new(), yolo),
        }
    }
}

impl Tool for ConfirmedBashTool {
    const NAME: &'static str = "bash";
    type Error = ToolError;
    type Args = BashArgs;
    type Output = BashOutput;

    async fn definition(&self, prompt: String) -> ToolDefinition {
        self.inner.inner.definition(prompt).await
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let description = format!("Execute command: {}", args.command);

        if !self.inner.confirm_execution(Self::NAME, &description)? {
            trace!("Bash tool execution denied by user");
            return Err(ToolError::PermissionDenied(
                "User denied tool execution".to_string(),
            ));
        }

        trace!("Executing bash command: {}", args.command);
        let result = self.inner.inner.call(args).await;

        match &result {
            Ok(_) => trace!("Bash command completed successfully"),
            Err(e) => trace!("Bash command failed: {}", e),
        }

        result
    }
}

/// Confirmed Edit File Tool
pub struct ConfirmedEditFileTool {
    inner: ConfirmedTool<EditFileTool>,
}

impl ConfirmedEditFileTool {
    pub fn new(yolo: bool) -> Self {
        Self {
            inner: ConfirmedTool::new(EditFileTool::new(), yolo),
        }
    }
}

impl Tool for ConfirmedEditFileTool {
    const NAME: &'static str = "edit_file";
    type Error = ToolError;
    type Args = EditFileArgs;
    type Output = EditFileOutput;

    async fn definition(&self, prompt: String) -> ToolDefinition {
        self.inner.inner.definition(prompt).await
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let description = format!("Edit/create file: {}", args.path);

        if !self.inner.confirm_execution(Self::NAME, &description)? {
            trace!("Edit file tool execution denied by user");
            return Err(ToolError::PermissionDenied(
                "User denied tool execution".to_string(),
            ));
        }

        trace!("Editing/creating file: {}", args.path);
        let result = self.inner.inner.call(args).await;

        match &result {
            Ok(_) => trace!("File edit completed successfully"),
            Err(e) => trace!("File edit failed: {}", e),
        }

        result
    }
}
