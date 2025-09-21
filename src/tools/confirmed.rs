use anyhow::Result;
use rig::completion::ToolDefinition;
use rig::tool::Tool;

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
        if self.yolo {
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
            return Err(ToolError::PermissionDenied(
                "User denied tool execution".to_string(),
            ));
        }

        self.inner.inner.call(args).await
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
            return Err(ToolError::PermissionDenied(
                "User denied tool execution".to_string(),
            ));
        }

        self.inner.inner.call(args).await
    }
}
