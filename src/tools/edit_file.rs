use anyhow::Result;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;
use tokio::fs;

use super::ToolError;

#[derive(Deserialize)]
pub struct EditFileArgs {
    pub path: String,
    pub content: String,
    #[serde(default)]
    pub create_if_missing: bool,
    #[serde(default)]
    pub backup: bool,
    #[serde(default)]
    pub encoding: Option<String>,
    #[serde(default)]
    pub line_range: Option<(usize, usize)>, // (start_line, end_line) for partial edits
}

#[derive(Serialize, Debug)]
pub struct EditFileOutput {
    pub path: String,
    pub success: bool,
    pub bytes_written: u64,
    pub backup_path: Option<String>,
    pub created_new_file: bool,
    pub lines_modified: Option<(usize, usize)>, // (start_line, end_line) if partial edit
}

#[derive(Deserialize, Serialize)]
pub struct EditFileTool;

impl EditFileTool {
    pub fn new() -> Self {
        Self
    }

    /// Edit file with safety checks and optional backup
    async fn edit_file_safe(&self, args: &EditFileArgs) -> Result<EditFileOutput, ToolError> {
        let path = Path::new(&args.path);
        let file_exists = path.exists();

        // Check if we can create the file if it doesn't exist
        if !file_exists && !args.create_if_missing {
            return Err(ToolError::FileNotFound(format!(
                "File '{}' does not exist and create_if_missing is false",
                args.path
            )));
        }

        // If file exists, check if it's actually a file
        if file_exists && !path.is_file() {
            return Err(ToolError::InvalidInput(format!(
                "Path '{}' exists but is not a file",
                args.path
            )));
        }

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .await
                    .map_err(|e| ToolError::Io(e))?;
            }
        }

        let mut backup_path = None;
        let mut original_content = String::new();

        // Read existing content if file exists
        if file_exists {
            original_content = fs::read_to_string(&path)
                .await
                .map_err(|e| ToolError::Io(e))?;

            // Create backup if requested
            if args.backup {
                let backup_file_path = format!("{}.backup", args.path);
                fs::copy(&path, &backup_file_path)
                    .await
                    .map_err(|e| ToolError::Io(e))?;
                backup_path = Some(backup_file_path);
            }
        }

        // Determine final content based on whether this is a partial edit
        let (final_content, lines_modified) = if let Some((start_line, end_line)) = args.line_range
        {
            // Partial edit: replace specific lines
            if !file_exists {
                return Err(ToolError::InvalidInput(
                    "Cannot perform line range edit on non-existent file".to_string(),
                ));
            }

            let lines: Vec<&str> = original_content.lines().collect();
            let total_lines = lines.len();

            if start_line == 0 || start_line > total_lines + 1 {
                return Err(ToolError::InvalidInput(format!(
                    "Invalid start line: {}. File has {} lines (1-indexed)",
                    start_line, total_lines
                )));
            }

            // Convert to 0-indexed
            let start_idx = start_line - 1;
            let end_idx = std::cmp::min(end_line, total_lines);

            // Split new content into lines
            let new_lines: Vec<&str> = args.content.lines().collect();

            // Replace the specified range
            let mut result_lines = Vec::new();
            result_lines.extend_from_slice(&lines[..start_idx]);
            result_lines.extend_from_slice(&new_lines);
            if end_idx < lines.len() {
                result_lines.extend_from_slice(&lines[end_idx..]);
            }

            let final_content = result_lines.join("\n");
            (
                final_content,
                Some((start_line, start_line + new_lines.len() - 1)),
            )
        } else {
            // Full file replacement
            (args.content.clone(), None)
        };

        // Write the content
        fs::write(&path, &final_content)
            .await
            .map_err(|e| ToolError::Io(e))?;

        // Get file size
        let metadata = fs::metadata(&path).await.map_err(|e| ToolError::Io(e))?;
        let bytes_written = metadata.len();

        Ok(EditFileOutput {
            path: args.path.clone(),
            success: true,
            bytes_written,
            backup_path,
            created_new_file: !file_exists,
            lines_modified,
        })
    }

    /// Validate file path for security
    fn validate_path(&self, path: &str) -> Result<(), ToolError> {
        let path = Path::new(path);

        // Check for path traversal attempts
        if path.to_string_lossy().contains("..") {
            return Err(ToolError::InvalidInput(
                "Path traversal (..) is not allowed".to_string(),
            ));
        }

        // Check for absolute paths to sensitive directories
        let sensitive_paths = [
            "/etc",
            "/usr/bin",
            "/usr/sbin",
            "/bin",
            "/sbin",
            "/System",
            "/Library",
            "/Applications",
        ];

        let path_str = path.to_string_lossy();
        for sensitive in &sensitive_paths {
            if path_str.starts_with(sensitive) {
                return Err(ToolError::PermissionDenied(format!(
                    "Access to {} is not allowed",
                    sensitive
                )));
            }
        }

        Ok(())
    }
}

impl Default for EditFileTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for EditFileTool {
    const NAME: &'static str = "edit_file";
    type Error = ToolError;
    type Args = EditFileArgs;
    type Output = EditFileOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Edits or creates a file with the specified content. Supports full file replacement or line range editing with optional backup.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path to the file to edit or create"
                    },
                    "content": {
                        "type": "string",
                        "description": "The content to write to the file"
                    },
                    "create_if_missing": {
                        "type": "boolean",
                        "description": "Whether to create the file if it doesn't exist (default: false)",
                        "default": false
                    },
                    "backup": {
                        "type": "boolean",
                        "description": "Whether to create a backup of the existing file (default: false)",
                        "default": false
                    },
                    "encoding": {
                        "type": "string",
                        "description": "Text encoding to use (default: UTF-8)"
                    },
                    "line_range": {
                        "type": "array",
                        "description": "Optional line range [start_line, end_line] to replace (1-indexed, inclusive)",
                        "items": {
                            "type": "number"
                        },
                        "minItems": 2,
                        "maxItems": 2
                    }
                },
                "required": ["path", "content"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate path for security
        self.validate_path(&args.path)?;

        self.edit_file_safe(&args).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

    #[test]
    fn test_edit_file_tool_creation() {
        let tool = EditFileTool::new();
        assert_eq!(EditFileTool::NAME, "edit_file");
    }

    #[tokio::test]
    async fn test_edit_file_definition() {
        let tool = EditFileTool::new();
        let definition = tool.definition("test prompt".to_string()).await;

        assert_eq!(definition.name, "edit_file");
        assert!(!definition.description.is_empty());
    }

    #[tokio::test]
    async fn test_create_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("new_file.txt");

        let tool = EditFileTool::new();
        let args = EditFileArgs {
            path: file_path.to_string_lossy().to_string(),
            content: "Hello, World!".to_string(),
            create_if_missing: true,
            backup: false,
            encoding: None,
            line_range: None,
        };

        let result = tool.call(args).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.success);
        assert!(output.created_new_file);
        assert!(output.backup_path.is_none());

        // Verify file was created with correct content
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_edit_existing_file_with_backup() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Original content").unwrap();

        let tool = EditFileTool::new();
        let args = EditFileArgs {
            path: temp_file.path().to_string_lossy().to_string(),
            content: "New content".to_string(),
            create_if_missing: false,
            backup: true,
            encoding: None,
            line_range: None,
        };

        let result = tool.call(args).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.success);
        assert!(!output.created_new_file);
        assert!(output.backup_path.is_some());

        // Verify new content
        let content = fs::read_to_string(temp_file.path()).await.unwrap();
        assert_eq!(content, "New content");

        // Verify backup exists
        let backup_path = output.backup_path.unwrap();
        let backup_content = fs::read_to_string(&backup_path).await.unwrap();
        assert!(backup_content.contains("Original content"));
    }

    #[tokio::test]
    async fn test_line_range_edit() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Line 1").unwrap();
        writeln!(temp_file, "Line 2").unwrap();
        writeln!(temp_file, "Line 3").unwrap();
        writeln!(temp_file, "Line 4").unwrap();

        let tool = EditFileTool::new();
        let args = EditFileArgs {
            path: temp_file.path().to_string_lossy().to_string(),
            content: "New Line 2\nNew Line 3".to_string(),
            create_if_missing: false,
            backup: false,
            encoding: None,
            line_range: Some((2, 3)),
        };

        let result = tool.call(args).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.success);
        assert_eq!(output.lines_modified, Some((2, 3)));

        // Verify content
        let content = fs::read_to_string(temp_file.path()).await.unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines[0], "Line 1");
        assert_eq!(lines[1], "New Line 2");
        assert_eq!(lines[2], "New Line 3");
        assert_eq!(lines[3], "Line 4");
    }

    #[test]
    fn test_validate_path_security() {
        let tool = EditFileTool::new();

        // Test path traversal
        assert!(tool.validate_path("../../../etc/passwd").is_err());
        assert!(tool.validate_path("./test/../../../etc/passwd").is_err());

        // Test sensitive paths
        assert!(tool.validate_path("/etc/passwd").is_err());
        assert!(tool.validate_path("/usr/bin/test").is_err());

        // Test valid paths
        assert!(tool.validate_path("./test.txt").is_ok());
        assert!(tool.validate_path("src/main.rs").is_ok());
        assert!(tool.validate_path("/tmp/test.txt").is_ok());
    }

    #[tokio::test]
    async fn test_create_file_without_permission() {
        let tool = EditFileTool::new();
        let args = EditFileArgs {
            path: "/nonexistent/path/file.txt".to_string(),
            content: "test".to_string(),
            create_if_missing: false,
            backup: false,
            encoding: None,
            line_range: None,
        };

        let result = tool.call(args).await;
        assert!(result.is_err());

        if let Err(ToolError::FileNotFound(_)) = result {
            // Expected error type
        } else {
            panic!("Expected FileNotFound error");
        }
    }
}
