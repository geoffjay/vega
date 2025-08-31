use anyhow::Result;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;
use tokio::fs;

use super::ToolError;

#[derive(Deserialize)]
pub struct ListFilesArgs {
    pub directory: String,
    #[serde(default)]
    pub recursive: bool,
    #[serde(default)]
    pub include_hidden: bool,
    #[serde(default)]
    pub file_types: Option<Vec<String>>, // e.g., ["rs", "toml", "md"]
    #[serde(default = "default_max_files")]
    pub max_files: usize,
    #[serde(default)]
    pub include_size: bool,
    #[serde(default)]
    pub include_modified: bool,
}

fn default_max_files() -> usize {
    1000
}

#[derive(Serialize, Debug)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size_bytes: Option<u64>,
    pub modified: Option<String>, // ISO 8601 timestamp
    pub extension: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct ListFilesOutput {
    pub files: Vec<FileInfo>,
    pub directory: String,
    pub total_files: usize,
    pub total_directories: usize,
    pub truncated: bool,
}

#[derive(Deserialize, Serialize)]
pub struct ListFilesTool;

impl ListFilesTool {
    pub fn new() -> Self {
        Self
    }

    /// List files in directory with filtering options
    async fn list_files_recursive(
        &self,
        args: &ListFilesArgs,
    ) -> Result<ListFilesOutput, ToolError> {
        let path = Path::new(&args.directory);

        // Check if directory exists
        if !path.exists() {
            return Err(ToolError::FileNotFound(args.directory.clone()));
        }

        // Check if it's actually a directory
        if !path.is_dir() {
            return Err(ToolError::InvalidInput(format!(
                "Path '{}' is not a directory",
                args.directory
            )));
        }

        let mut all_files = Vec::new();
        let mut total_files = 0;
        let mut total_directories = 0;

        if args.recursive {
            self.collect_files_recursive(
                path,
                &mut all_files,
                args,
                &mut total_files,
                &mut total_directories,
            )
            .await?;
        } else {
            self.collect_files_single_level(
                path,
                &mut all_files,
                args,
                &mut total_files,
                &mut total_directories,
            )
            .await?;
        }

        // Sort files by name
        all_files.sort_by(|a, b| a.name.cmp(&b.name));

        // Apply max files limit
        let truncated = all_files.len() > args.max_files;
        if truncated {
            all_files.truncate(args.max_files);
        }

        Ok(ListFilesOutput {
            files: all_files,
            directory: args.directory.clone(),
            total_files,
            total_directories,
            truncated,
        })
    }

    /// Collect files from a single directory level
    async fn collect_files_single_level(
        &self,
        dir_path: &Path,
        files: &mut Vec<FileInfo>,
        args: &ListFilesArgs,
        total_files: &mut usize,
        total_directories: &mut usize,
    ) -> Result<(), ToolError> {
        let mut entries = fs::read_dir(dir_path).await.map_err(|e| ToolError::Io(e))?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| ToolError::Io(e))? {
            if files.len() >= args.max_files {
                break;
            }

            let file_info = self.create_file_info(&entry, args).await?;

            if let Some(info) = file_info {
                if info.is_directory {
                    *total_directories += 1;
                } else {
                    *total_files += 1;
                }
                files.push(info);
            }
        }

        Ok(())
    }

    /// Collect files recursively
    fn collect_files_recursive<'a>(
        &'a self,
        dir_path: &'a Path,
        files: &'a mut Vec<FileInfo>,
        args: &'a ListFilesArgs,
        total_files: &'a mut usize,
        total_directories: &'a mut usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ToolError>> + 'a + Send>>
    {
        Box::pin(async move {
            let mut entries = fs::read_dir(dir_path).await.map_err(|e| ToolError::Io(e))?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| ToolError::Io(e))? {
                if files.len() >= args.max_files {
                    break;
                }

                let file_info = self.create_file_info(&entry, args).await?;

                if let Some(info) = file_info {
                    let is_dir = info.is_directory;

                    if is_dir {
                        *total_directories += 1;
                    } else {
                        *total_files += 1;
                    }

                    files.push(info);

                    // Recurse into subdirectories
                    if is_dir && files.len() < args.max_files {
                        let sub_path = entry.path();
                        if let Err(e) = self
                            .collect_files_recursive(
                                &sub_path,
                                files,
                                args,
                                total_files,
                                total_directories,
                            )
                            .await
                        {
                            // Log error but continue with other directories
                            eprintln!(
                                "Warning: Failed to read directory {}: {}",
                                sub_path.display(),
                                e
                            );
                        }
                    }
                }
            }

            Ok(())
        })
    }

    /// Create FileInfo from directory entry
    async fn create_file_info(
        &self,
        entry: &fs::DirEntry,
        args: &ListFilesArgs,
    ) -> Result<Option<FileInfo>, ToolError> {
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files if not requested
        if !args.include_hidden && file_name.starts_with('.') {
            return Ok(None);
        }

        let metadata = entry.metadata().await.map_err(|e| ToolError::Io(e))?;

        let is_directory = metadata.is_dir();

        // Get file extension
        let extension = if !is_directory {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|s| s.to_lowercase())
        } else {
            None
        };

        // Filter by file types if specified
        if let Some(ref file_types) = args.file_types {
            if !is_directory {
                if let Some(ref ext) = extension {
                    if !file_types.iter().any(|ft| ft.to_lowercase() == *ext) {
                        return Ok(None);
                    }
                } else {
                    // No extension, skip if file types are specified
                    return Ok(None);
                }
            }
        }

        // Get file size if requested
        let size_bytes = if args.include_size && !is_directory {
            Some(metadata.len())
        } else {
            None
        };

        // Get modification time if requested
        let modified = if args.include_modified {
            metadata.modified().ok().and_then(|time| {
                time.duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .map(|duration| {
                        let datetime =
                            chrono::DateTime::from_timestamp(duration.as_secs() as i64, 0)?;
                        Some(datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string())
                    })
                    .flatten()
            })
        } else {
            None
        };

        Ok(Some(FileInfo {
            name: file_name,
            path: path.to_string_lossy().to_string(),
            is_directory,
            size_bytes,
            modified,
            extension,
        }))
    }
}

impl Default for ListFilesTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for ListFilesTool {
    const NAME: &'static str = "list_files";
    type Error = ToolError;
    type Args = ListFilesArgs;
    type Output = ListFilesOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Lists files and directories in a specified directory with filtering and metadata options.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "directory": {
                        "type": "string",
                        "description": "The directory path to list files from"
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "Whether to list files recursively (default: false)",
                        "default": false
                    },
                    "include_hidden": {
                        "type": "boolean",
                        "description": "Whether to include hidden files (starting with .) (default: false)",
                        "default": false
                    },
                    "file_types": {
                        "type": "array",
                        "description": "File extensions to filter by (e.g., ['rs', 'toml', 'md'])",
                        "items": {
                            "type": "string"
                        }
                    },
                    "max_files": {
                        "type": "number",
                        "description": "Maximum number of files to return (default: 1000)",
                        "default": 1000
                    },
                    "include_size": {
                        "type": "boolean",
                        "description": "Whether to include file sizes (default: false)",
                        "default": false
                    },
                    "include_modified": {
                        "type": "boolean",
                        "description": "Whether to include modification timestamps (default: false)",
                        "default": false
                    }
                },
                "required": ["directory"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.list_files_recursive(&args).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_list_files_tool_creation() {
        let tool = ListFilesTool::new();
        assert_eq!(ListFilesTool::NAME, "list_files");
    }

    #[test]
    fn test_default_max_files() {
        assert_eq!(default_max_files(), 1000);
    }

    #[tokio::test]
    async fn test_list_files_definition() {
        let tool = ListFilesTool::new();
        let definition = tool.definition("test prompt".to_string()).await;

        assert_eq!(definition.name, "list_files");
        assert!(!definition.description.is_empty());
    }

    #[tokio::test]
    async fn test_list_files_single_level() {
        let temp_dir = TempDir::new().unwrap();

        // Create some test files
        std::fs::File::create(temp_dir.path().join("file1.txt")).unwrap();
        std::fs::File::create(temp_dir.path().join("file2.rs")).unwrap();
        std::fs::create_dir(temp_dir.path().join("subdir")).unwrap();

        let tool = ListFilesTool::new();
        let args = ListFilesArgs {
            directory: temp_dir.path().to_string_lossy().to_string(),
            recursive: false,
            include_hidden: false,
            file_types: None,
            max_files: 100,
            include_size: false,
            include_modified: false,
        };

        let result = tool.call(args).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.files.len(), 3);
        assert_eq!(output.total_files, 2);
        assert_eq!(output.total_directories, 1);
        assert!(!output.truncated);
    }

    #[tokio::test]
    async fn test_list_files_with_file_type_filter() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files with different extensions
        std::fs::File::create(temp_dir.path().join("file1.txt")).unwrap();
        std::fs::File::create(temp_dir.path().join("file2.rs")).unwrap();
        std::fs::File::create(temp_dir.path().join("file3.md")).unwrap();

        let tool = ListFilesTool::new();
        let args = ListFilesArgs {
            directory: temp_dir.path().to_string_lossy().to_string(),
            recursive: false,
            include_hidden: false,
            file_types: Some(vec!["rs".to_string(), "md".to_string()]),
            max_files: 100,
            include_size: false,
            include_modified: false,
        };

        let result = tool.call(args).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.files.len(), 2); // Only .rs and .md files

        let extensions: Vec<_> = output
            .files
            .iter()
            .filter_map(|f| f.extension.as_ref())
            .collect();
        assert!(extensions.contains(&&"rs".to_string()));
        assert!(extensions.contains(&&"md".to_string()));
    }

    #[tokio::test]
    async fn test_list_files_recursive() {
        let temp_dir = TempDir::new().unwrap();

        // Create nested directory structure
        let subdir = temp_dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();

        std::fs::File::create(temp_dir.path().join("root.txt")).unwrap();
        std::fs::File::create(subdir.join("nested.txt")).unwrap();

        let tool = ListFilesTool::new();
        let args = ListFilesArgs {
            directory: temp_dir.path().to_string_lossy().to_string(),
            recursive: true,
            include_hidden: false,
            file_types: None,
            max_files: 100,
            include_size: false,
            include_modified: false,
        };

        let result = tool.call(args).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.files.len(), 3); // root.txt, subdir, nested.txt
        assert_eq!(output.total_files, 2);
        assert_eq!(output.total_directories, 1);
    }

    #[tokio::test]
    async fn test_list_files_with_metadata() {
        let temp_dir = TempDir::new().unwrap();

        // Create a test file with some content
        let file_path = temp_dir.path().join("test.txt");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "Hello, World!").unwrap();

        let tool = ListFilesTool::new();
        let args = ListFilesArgs {
            directory: temp_dir.path().to_string_lossy().to_string(),
            recursive: false,
            include_hidden: false,
            file_types: None,
            max_files: 100,
            include_size: true,
            include_modified: true,
        };

        let result = tool.call(args).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.files.len(), 1);

        let file_info = &output.files[0];
        assert!(file_info.size_bytes.is_some());
        assert!(file_info.size_bytes.unwrap() > 0);
        assert!(file_info.modified.is_some());
    }

    #[tokio::test]
    async fn test_list_nonexistent_directory() {
        let tool = ListFilesTool::new();
        let args = ListFilesArgs {
            directory: "/nonexistent/directory".to_string(),
            recursive: false,
            include_hidden: false,
            file_types: None,
            max_files: 100,
            include_size: false,
            include_modified: false,
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
