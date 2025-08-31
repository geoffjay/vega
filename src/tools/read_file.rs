use anyhow::Result;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;
use tokio::fs;

use super::ToolError;

#[derive(Deserialize)]
pub struct ReadFileArgs {
    pub path: String,
    #[serde(default)]
    pub encoding: Option<String>,
    #[serde(default)]
    pub max_size_mb: Option<u64>,
    #[serde(default)]
    pub line_range: Option<(usize, usize)>, // (start_line, end_line) - 1-indexed
}

#[derive(Serialize, Debug)]
pub struct ReadFileOutput {
    pub content: String,
    pub path: String,
    pub size_bytes: u64,
    pub line_count: usize,
    pub encoding_used: String,
    pub is_binary: bool,
    pub truncated: bool,
}

#[derive(Deserialize, Serialize)]
pub struct ReadFileTool;

impl ReadFileTool {
    pub fn new() -> Self {
        Self
    }

    /// Read file with safety checks and optional line range
    async fn read_file_safe(&self, args: &ReadFileArgs) -> Result<ReadFileOutput, ToolError> {
        let path = Path::new(&args.path);

        // Check if file exists
        if !path.exists() {
            return Err(ToolError::FileNotFound(args.path.clone()));
        }

        // Check if it's a file (not a directory)
        if !path.is_file() {
            return Err(ToolError::InvalidInput(format!(
                "Path '{}' is not a file",
                args.path
            )));
        }

        // Get file metadata
        let metadata = fs::metadata(&path).await.map_err(|e| ToolError::Io(e))?;

        let file_size = metadata.len();

        // Check file size limits (default 10MB)
        let max_size_bytes = args.max_size_mb.unwrap_or(10) * 1024 * 1024;
        if file_size > max_size_bytes {
            return Err(ToolError::InvalidInput(format!(
                "File size ({} bytes) exceeds maximum allowed size ({} bytes)",
                file_size, max_size_bytes
            )));
        }

        // Read file content
        let content_bytes = fs::read(&path).await.map_err(|e| ToolError::Io(e))?;

        // Check if file is binary
        let is_binary = self.is_binary_content(&content_bytes);

        let (content, encoding_used) = if is_binary {
            // For binary files, provide a hex dump of first 1KB
            let preview_size = std::cmp::min(content_bytes.len(), 1024);
            let hex_content = (&content_bytes)[..preview_size]
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .chunks(16)
                .map(|chunk| chunk.join(" "))
                .collect::<Vec<_>>()
                .join("\n");

            let content = if content_bytes.len() > preview_size {
                format!(
                    "{}\n... (binary file truncated, showing first {} bytes as hex)",
                    hex_content, preview_size
                )
            } else {
                format!(
                    "{}\n(binary file, {} bytes shown as hex)",
                    hex_content,
                    content_bytes.len()
                )
            };

            (content, "binary-hex".to_string())
        } else {
            // Try to decode as UTF-8
            match String::from_utf8(content_bytes.clone()) {
                Ok(text) => (text, "utf-8".to_string()),
                Err(_) => {
                    // Try to decode as latin-1 (which can decode any byte sequence)
                    let text = content_bytes.iter().map(|&b| b as char).collect::<String>();
                    (text, "latin-1".to_string())
                }
            }
        };

        // Apply line range filtering if specified
        let (final_content, line_count, truncated) =
            if let Some((start_line, end_line)) = args.line_range {
                let lines: Vec<&str> = content.lines().collect();
                let total_lines = lines.len();

                if start_line == 0 || start_line > total_lines {
                    return Err(ToolError::InvalidInput(format!(
                        "Invalid start line: {}. File has {} lines (1-indexed)",
                        start_line, total_lines
                    )));
                }

                let start_idx = start_line - 1; // Convert to 0-indexed
                let end_idx = std::cmp::min(end_line, total_lines);

                let selected_lines = &lines[start_idx..end_idx];
                let range_content = selected_lines.join("\n");

                (range_content, total_lines, end_line < total_lines)
            } else {
                let line_count = content.lines().count();
                (content, line_count, false)
            };

        Ok(ReadFileOutput {
            content: final_content,
            path: args.path.clone(),
            size_bytes: file_size,
            line_count,
            encoding_used,
            is_binary,
            truncated,
        })
    }

    /// Simple heuristic to detect binary content
    fn is_binary_content(&self, content: &[u8]) -> bool {
        // Check for null bytes or high ratio of non-printable characters
        let null_count = content.iter().filter(|&&b| b == 0).count();
        if null_count > 0 {
            return true;
        }

        // Check ratio of printable ASCII characters
        let printable_count = content
            .iter()
            .filter(|&&b| b >= 32 && b <= 126 || b == 9 || b == 10 || b == 13)
            .count();

        let printable_ratio = printable_count as f64 / content.len() as f64;
        printable_ratio < 0.7 // If less than 70% printable, consider binary
    }
}

impl Default for ReadFileTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for ReadFileTool {
    const NAME: &'static str = "read_file";
    type Error = ToolError;
    type Args = ReadFileArgs;
    type Output = ReadFileOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Reads the contents of a file from the filesystem with safety checks and optional line range selection.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path to the file to read"
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
                        "description": "Optional line range [start_line, end_line] (1-indexed, inclusive)",
                        "items": {
                            "type": "number"
                        },
                        "minItems": 2,
                        "maxItems": 2
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.read_file_safe(&args).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_read_file_tool_creation() {
        let tool = ReadFileTool::new();
        assert_eq!(ReadFileTool::NAME, "read_file");
    }

    #[tokio::test]
    async fn test_read_file_definition() {
        let tool = ReadFileTool::new();
        let definition = tool.definition("test prompt".to_string()).await;

        assert_eq!(definition.name, "read_file");
        assert!(!definition.description.is_empty());
    }

    #[tokio::test]
    async fn test_read_text_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Hello, World!").unwrap();
        writeln!(temp_file, "This is a test file.").unwrap();

        let tool = ReadFileTool::new();
        let args = ReadFileArgs {
            path: temp_file.path().to_string_lossy().to_string(),
            encoding: None,
            max_size_mb: None,
            line_range: None,
        };

        let result = tool.call(args).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(!output.is_binary);
        assert_eq!(output.encoding_used, "utf-8");
        assert!(output.content.contains("Hello, World!"));
        assert_eq!(output.line_count, 2);
    }

    #[tokio::test]
    async fn test_read_file_with_line_range() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Line 1").unwrap();
        writeln!(temp_file, "Line 2").unwrap();
        writeln!(temp_file, "Line 3").unwrap();
        writeln!(temp_file, "Line 4").unwrap();

        let tool = ReadFileTool::new();
        let args = ReadFileArgs {
            path: temp_file.path().to_string_lossy().to_string(),
            encoding: None,
            max_size_mb: None,
            line_range: Some((2, 3)),
        };

        let result = tool.call(args).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.line_count, 4); // Total lines in file
        assert!(output.content.contains("Line 2"));
        assert!(output.content.contains("Line 3"));
        assert!(!output.content.contains("Line 1"));
        assert!(!output.content.contains("Line 4"));
    }

    #[tokio::test]
    async fn test_read_nonexistent_file() {
        let tool = ReadFileTool::new();
        let args = ReadFileArgs {
            path: "/nonexistent/file.txt".to_string(),
            encoding: None,
            max_size_mb: None,
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

    #[test]
    fn test_is_binary_content() {
        let tool = ReadFileTool::new();

        // Text content
        let text_content = b"Hello, World!\nThis is text.";
        assert!(!tool.is_binary_content(text_content));

        // Binary content with null bytes
        let binary_content = b"Hello\x00World";
        assert!(tool.is_binary_content(binary_content));

        // Content with many non-printable characters
        let non_printable: Vec<u8> = (0..255).collect();
        assert!(tool.is_binary_content(&non_printable));
    }
}
