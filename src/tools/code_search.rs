use anyhow::Result;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::process::Command;

use super::ToolError;

#[derive(Deserialize)]
pub struct CodeSearchArgs {
    pub pattern: String,
    pub path: String,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub whole_word: bool,
    #[serde(default)]
    pub file_type: Option<String>,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    #[serde(default)]
    pub context_lines: Option<usize>,
}

fn default_max_results() -> usize {
    50
}

#[derive(Serialize, Debug)]
pub struct CodeSearchMatch {
    pub file_path: String,
    pub line_number: usize,
    pub line_content: String,
    pub column: Option<usize>,
}

#[derive(Serialize, Debug)]
pub struct CodeSearchOutput {
    pub matches: Vec<CodeSearchMatch>,
    pub pattern: String,
    pub path: String,
    pub total_matches: usize,
    pub files_searched: usize,
}

#[derive(Deserialize, Serialize)]
pub struct CodeSearchTool;

impl CodeSearchTool {
    pub fn new() -> Self {
        Self
    }

    /// Execute ripgrep search with the given parameters
    async fn search_with_ripgrep(
        &self,
        args: &CodeSearchArgs,
    ) -> Result<CodeSearchOutput, ToolError> {
        // Check if ripgrep is available
        let rg_available = Command::new("rg").arg("--version").output().is_ok();

        if !rg_available {
            return Err(ToolError::Command(
                "ripgrep (rg) is not installed or not available in PATH".to_string(),
            ));
        }

        // Build the ripgrep command
        let mut cmd = Command::new("rg");

        // Add the pattern
        cmd.arg(&args.pattern);

        // Add the path
        cmd.arg(&args.path);

        // Add flags
        cmd.arg("--line-number");
        cmd.arg("--column");
        cmd.arg("--no-heading");
        cmd.arg("--with-filename");

        // Case sensitivity
        if !args.case_sensitive {
            cmd.arg("--ignore-case");
        }

        // Whole word matching
        if args.whole_word {
            cmd.arg("--word-regexp");
        }

        // File type filtering
        if let Some(ref file_type) = args.file_type {
            cmd.arg("--type").arg(file_type);
        }

        // Context lines
        if let Some(context) = args.context_lines {
            cmd.arg("--context").arg(context.to_string());
        }

        // Max count (approximate, ripgrep doesn't have exact match limit)
        cmd.arg("--max-count").arg(args.max_results.to_string());

        // Execute the command
        let output = tokio::task::spawn_blocking(move || cmd.output())
            .await
            .map_err(|e| ToolError::Command(format!("Failed to spawn ripgrep: {}", e)))?
            .map_err(|e| ToolError::Command(format!("Ripgrep execution failed: {}", e)))?;

        // Parse the output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut matches = Vec::new();
        let mut files_searched = std::collections::HashSet::new();

        for line in stdout.lines() {
            if let Some(search_match) = self.parse_ripgrep_line(line) {
                files_searched.insert(search_match.file_path.clone());
                matches.push(search_match);

                if matches.len() >= args.max_results {
                    break;
                }
            }
        }

        Ok(CodeSearchOutput {
            total_matches: matches.len(),
            files_searched: files_searched.len(),
            pattern: args.pattern.clone(),
            path: args.path.clone(),
            matches,
        })
    }

    /// Parse a single line of ripgrep output
    fn parse_ripgrep_line(&self, line: &str) -> Option<CodeSearchMatch> {
        // Ripgrep output format: file:line:column:content
        let parts: Vec<&str> = line.splitn(4, ':').collect();

        if parts.len() >= 3 {
            let file_path = parts[0].to_string();
            let line_number = parts[1].parse::<usize>().ok()?;
            let column = if parts.len() >= 4 {
                parts[2].parse::<usize>().ok()
            } else {
                None
            };
            let line_content = if parts.len() >= 4 {
                parts[3].to_string()
            } else {
                parts[2].to_string()
            };

            Some(CodeSearchMatch {
                file_path,
                line_number,
                line_content,
                column,
            })
        } else {
            None
        }
    }
}

impl Default for CodeSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for CodeSearchTool {
    const NAME: &'static str = "code_search";
    type Error = ToolError;
    type Args = CodeSearchArgs;
    type Output = CodeSearchOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Searches code using ripgrep with support for regex patterns, file type filtering, and context lines.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The regex pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "The file or directory path to search in"
                    },
                    "case_sensitive": {
                        "type": "boolean",
                        "description": "Whether the search should be case sensitive (default: false)",
                        "default": false
                    },
                    "whole_word": {
                        "type": "boolean",
                        "description": "Whether to match whole words only (default: false)",
                        "default": false
                    },
                    "file_type": {
                        "type": "string",
                        "description": "File type to filter by (e.g., 'rust', 'js', 'py')"
                    },
                    "max_results": {
                        "type": "number",
                        "description": "Maximum number of results to return (default: 50)",
                        "default": 50
                    },
                    "context_lines": {
                        "type": "number",
                        "description": "Number of context lines to show around matches"
                    }
                },
                "required": ["pattern", "path"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.search_with_ripgrep(&args).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_search_tool_creation() {
        let tool = CodeSearchTool::new();
        assert_eq!(CodeSearchTool::NAME, "code_search");
    }

    #[test]
    fn test_default_max_results() {
        assert_eq!(default_max_results(), 50);
    }

    #[tokio::test]
    async fn test_code_search_definition() {
        let tool = CodeSearchTool::new();
        let definition = tool.definition("test prompt".to_string()).await;

        assert_eq!(definition.name, "code_search");
        assert!(!definition.description.is_empty());
    }

    #[test]
    fn test_parse_ripgrep_line() {
        let tool = CodeSearchTool::new();

        // Test with column information
        let line = "src/main.rs:42:15:    let result = calculate();";
        let parsed = tool.parse_ripgrep_line(line);

        assert!(parsed.is_some());
        let search_match = parsed.unwrap();
        assert_eq!(search_match.file_path, "src/main.rs");
        assert_eq!(search_match.line_number, 42);
        assert_eq!(search_match.column, Some(15));
        assert_eq!(search_match.line_content, "    let result = calculate();");
    }

    #[test]
    fn test_parse_ripgrep_line_without_column() {
        let tool = CodeSearchTool::new();

        // Test without column information
        let line = "src/lib.rs:10:pub fn main() {";
        let parsed = tool.parse_ripgrep_line(line);

        assert!(parsed.is_some());
        let search_match = parsed.unwrap();
        assert_eq!(search_match.file_path, "src/lib.rs");
        assert_eq!(search_match.line_number, 10);
        assert_eq!(search_match.line_content, "pub fn main() {");
    }
}
