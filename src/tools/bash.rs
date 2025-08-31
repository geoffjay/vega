use anyhow::Result;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::process::Command;

use super::ToolError;

#[derive(Deserialize)]
pub struct BashArgs {
    pub command: String,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default)]
    pub working_directory: Option<String>,
}

fn default_timeout() -> u64 {
    30 // 30 seconds default timeout
}

#[derive(Serialize, Debug)]
pub struct BashOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub command: String,
    pub success: bool,
}

#[derive(Deserialize, Serialize)]
pub struct BashTool;

impl BashTool {
    pub fn new() -> Self {
        Self
    }

    /// Execute a shell command with timeout and safety checks
    async fn execute_command(&self, args: &BashArgs) -> Result<BashOutput, ToolError> {
        // Basic safety checks - prevent obviously dangerous commands
        let dangerous_patterns = [
            "rm -rf /",
            ":(){ :|:& };:", // fork bomb
            "dd if=/dev/zero",
            "mkfs",
            "format",
            "> /dev/",
            "shutdown",
            "reboot",
            "halt",
        ];

        let command_lower = args.command.to_lowercase();
        for pattern in &dangerous_patterns {
            if command_lower.contains(pattern) {
                return Err(ToolError::InvalidInput(format!(
                    "Command contains potentially dangerous pattern: {}",
                    pattern
                )));
            }
        }

        // Create the command
        let mut cmd = if cfg!(target_os = "windows") {
            let mut cmd = Command::new("cmd");
            cmd.args(["/C", &args.command]);
            cmd
        } else {
            let mut cmd = Command::new("sh");
            cmd.args(["-c", &args.command]);
            cmd
        };

        // Set working directory if provided
        if let Some(ref dir) = args.working_directory {
            cmd.current_dir(dir);
        }

        // Execute the command
        let output = tokio::task::spawn_blocking(move || cmd.output())
            .await
            .map_err(|e| ToolError::Command(format!("Failed to spawn command: {}", e)))?
            .map_err(|e| ToolError::Command(format!("Command execution failed: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);
        let success = output.status.success();

        Ok(BashOutput {
            stdout,
            stderr,
            exit_code,
            command: args.command.clone(),
            success,
        })
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for BashTool {
    const NAME: &'static str = "bash";
    type Error = ToolError;
    type Args = BashArgs;
    type Output = BashOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Executes shell commands and returns the output. Includes basic safety checks to prevent dangerous operations.".to_string(),
            parameters: json!({
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
                        "description": "Working directory for the command (optional)"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.execute_command(&args).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bash_tool_creation() {
        let tool = BashTool::new();
        assert_eq!(BashTool::NAME, "bash");
    }

    #[test]
    fn test_default_timeout() {
        assert_eq!(default_timeout(), 30);
    }

    #[tokio::test]
    async fn test_bash_definition() {
        let tool = BashTool::new();
        let definition = tool.definition("test prompt".to_string()).await;

        assert_eq!(definition.name, "bash");
        assert!(!definition.description.is_empty());
    }

    #[tokio::test]
    async fn test_safe_command() {
        let tool = BashTool::new();
        let args = BashArgs {
            command: "echo 'hello world'".to_string(),
            timeout_seconds: 5,
            working_directory: None,
        };

        let result = tool.call(args).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.success);
        assert!(output.stdout.contains("hello world"));
    }

    #[tokio::test]
    async fn test_dangerous_command_blocked() {
        let tool = BashTool::new();
        let args = BashArgs {
            command: "rm -rf /".to_string(),
            timeout_seconds: 5,
            working_directory: None,
        };

        let result = tool.call(args).await;
        assert!(result.is_err());

        if let Err(ToolError::InvalidInput(msg)) = result {
            assert!(msg.contains("dangerous pattern"));
        } else {
            panic!("Expected InvalidInput error");
        }
    }
}
