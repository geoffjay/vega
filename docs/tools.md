# Ally Tools Documentation

This document describes all available tools in the Ally AI agent system and their purposes.

## Overview

Ally provides a comprehensive set of tools that enable the AI agent to interact with the system, manipulate files, search code, browse the web, and more. All tools are implemented using the Rig framework's tool system and include safety checks and validation.

## Available Tools

### 1. Bash Tool (`bash`)

**Purpose**: Execute shell commands with safety checks and timeout protection.

**Key Features**:

- Cross-platform support (Windows and Unix-like systems)
- Built-in safety checks to prevent dangerous operations
- Configurable timeout (default: 30 seconds)
- Working directory support
- Comprehensive output capture (stdout, stderr, exit code)

**Safety Features**:

- Blocks dangerous patterns like `rm -rf /`, fork bombs, disk formatting commands
- Prevents access to system directories
- Timeout protection to prevent hanging processes

**Parameters**:

- `command` (required): The shell command to execute
- `timeout_seconds` (optional): Timeout in seconds (default: 30)
- `working_directory` (optional): Working directory for the command

**Example Use Cases**:

- Running build commands (`cargo build`, `npm install`)
- File system operations (`ls`, `find`, `grep`)
- Git operations (`git status`, `git log`)
- System information gathering (`ps`, `df`, `uname`)

### 2. Code Search Tool (`code_search`)

**Purpose**: Search through code using ripgrep with advanced pattern matching and filtering.

**Key Features**:

- Regex pattern support
- File type filtering
- Case-sensitive/insensitive search
- Whole word matching
- Context lines around matches
- Line and column number reporting

**Parameters**:

- `pattern` (required): The regex pattern to search for
- `path` (required): File or directory path to search in
- `case_sensitive` (optional): Whether search should be case sensitive (default: false)
- `whole_word` (optional): Whether to match whole words only (default: false)
- `file_type` (optional): File type filter (e.g., 'rust', 'js', 'py')
- `max_results` (optional): Maximum number of results (default: 50)
- `context_lines` (optional): Number of context lines around matches

**Example Use Cases**:

- Finding function definitions: `fn\s+calculate_total`
- Searching for imports: `use\s+std::`
- Finding TODO comments: `TODO|FIXME`
- Locating error handling: `Result<.*Error>`

### 3. Web Search Tool (`web_search`)

**Purpose**: Perform web searches using DuckDuckGo's API to find current information.

**Key Features**:

- DuckDuckGo instant answers integration
- Related topics extraction
- Fallback to manual search URLs
- Configurable result limits

**Parameters**:

- `query` (required): The search query string
- `max_results` (optional): Maximum number of results (default: 5)

**Example Use Cases**:

- Finding documentation: "Rust async programming guide"
- Looking up error messages: "cargo build error E0277"
- Researching libraries: "best HTTP client for Rust"
- Getting current information: "latest Rust version features"

### 4. File Operations Tools

#### Read File Tool (`read_file`)

**Purpose**: Read file contents with safety checks and encoding detection.

**Key Features**:

- Binary file detection and hex dump preview
- Encoding detection (UTF-8, Latin-1)
- File size limits (default: 10MB)
- Line range selection
- Metadata reporting (size, line count, encoding)

**Parameters**:

- `path` (required): Path to the file to read
- `encoding` (optional): Text encoding to use (auto-detected if not specified)
- `max_size_mb` (optional): Maximum file size in MB (default: 10)
- `line_range` (optional): Line range [start_line, end_line] (1-indexed)

#### Edit File Tool (`edit_file`)

**Purpose**: Create or edit files with backup support and safety validation.

**Key Features**:

- Automatic file creation if missing
- Optional backup creation
- Line range editing for partial updates
- Path traversal protection
- Parent directory creation

**Parameters**:

- `path` (required): Path to the file to edit or create
- `content` (required): Content to write to the file
- `create_if_missing` (optional): Create file if it doesn't exist (default: false)
- `backup` (optional): Create backup of existing file (default: false)
- `encoding` (optional): Text encoding to use (default: UTF-8)
- `line_range` (optional): Line range [start_line, end_line] for partial edits

#### List Files Tool (`list_files`)

**Purpose**: List directory contents with filtering and metadata options.

**Key Features**:

- Recursive directory traversal
- File type filtering by extension
- Hidden file inclusion/exclusion
- Metadata collection (size, modification time)
- Result limiting and sorting

**Parameters**:

- `directory` (required): Directory path to list files from
- `recursive` (optional): List files recursively (default: false)
- `include_hidden` (optional): Include hidden files (default: false)
- `file_types` (optional): File extensions to filter by (e.g., ['rs', 'toml'])
- `max_files` (optional): Maximum number of files to return (default: 1000)
- `include_size` (optional): Include file sizes (default: false)
- `include_modified` (optional): Include modification timestamps (default: false)

### 5. Read Logs Tool (`read_logs`)

**Purpose**: Read and filter log entries for specific sessions.

**Key Features**:

- Session-specific log filtering
- Multiple log source support (file, vector store)
- Log level filtering (error, warn, info, debug, trace)
- Timestamp-based sorting
- Configurable result limits

**Parameters**:

- `session_id` (required): Session ID to read logs for
- `limit` (optional): Maximum number of log entries (default: 50)
- `level_filter` (optional): Filter by log level (error, warn, info, debug, trace)

**Example Use Cases**:

- Debugging session issues: Read error logs for a specific session
- Performance analysis: Review debug logs with timing information
- Audit trails: Check info logs for user actions
- System monitoring: Filter warning and error logs

### 6. Confirmed Tools

The system also provides "confirmed" versions of potentially destructive tools that require user approval before execution (unless running in YOLO mode).

#### Confirmed Bash Tool (`ConfirmedBashTool`)

- Wraps the bash tool with user confirmation prompts
- Same functionality as bash tool but with safety confirmation

#### Confirmed Edit File Tool (`ConfirmedEditFileTool`)

- Wraps the edit file tool with user confirmation prompts
- Same functionality as edit file tool but with safety confirmation

## Tool Safety and Security

### Security Measures

1. **Path Validation**: All file operations validate paths to prevent:

   - Path traversal attacks (`../../../etc/passwd`)
   - Access to sensitive system directories (`/etc`, `/usr/bin`, etc.)

2. **Command Safety**: The bash tool includes:

   - Dangerous command pattern detection
   - Timeout protection
   - Output size limits

3. **File Size Limits**: File operations respect:

   - Maximum file size limits (configurable)
   - Memory usage protection
   - Binary file handling

4. **User Confirmation**: Confirmed tools provide:
   - Interactive approval prompts
   - YOLO mode bypass for automation
   - Clear action descriptions

### Error Handling

All tools use a common error type system (`ToolError`) that provides:

- `Io`: File system and I/O errors
- `Http`: Network and HTTP errors
- `Json`: JSON parsing errors
- `Command`: Command execution failures
- `FileNotFound`: Missing file errors
- `PermissionDenied`: Access permission errors
- `InvalidInput`: Input validation errors

## Configuration

### Environment Variables

Tools respect various environment variables for configuration:

- `ALLY_LOG_OUTPUT`: Controls log output destination (console, file, vector)
- `ALLY_LOG_FILE`: Path to log file for file-based logging
- Tool-specific timeouts and limits can be configured via parameters

### Tool Collection

Tools are organized in a collection that can be easily extended:

```rust
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
```

## Best Practices

### When to Use Each Tool

1. **Use `bash` for**:

   - System commands and utilities
   - Build and deployment tasks
   - File system operations not covered by specific tools
   - Git operations and version control

2. **Use `code_search` for**:

   - Finding specific code patterns
   - Locating function definitions
   - Searching for imports or dependencies
   - Code analysis and refactoring

3. **Use `web_search` for**:

   - Finding current documentation
   - Researching error messages
   - Looking up best practices
   - Getting up-to-date information

4. **Use file tools for**:

   - `read_file`: Examining configuration files, source code, logs
   - `edit_file`: Making targeted changes to files
   - `list_files`: Understanding project structure, finding files

5. **Use `read_logs` for**:
   - Debugging session-specific issues
   - Performance analysis
   - Audit and compliance
   - System monitoring

### Safety Guidelines

1. **Always validate inputs** before using tools
2. **Use confirmed tools** for potentially destructive operations
3. **Set appropriate limits** (timeouts, file sizes, result counts)
4. **Handle errors gracefully** and provide meaningful feedback
5. **Respect system resources** and avoid excessive operations

## Integration with Rig Framework

All tools implement the Rig framework's `Tool` trait, providing:

- Consistent interface and error handling
- JSON schema-based parameter validation
- Async execution support
- Automatic serialization/deserialization

This ensures tools can be easily integrated with different LLM providers and agent frameworks while maintaining type safety and validation.
