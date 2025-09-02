# AGENTS.md

## Project Overview

This is the Ally AI agent project, built with Rust and the Rig framework. It provides an interactive chat interface with tool support for various tasks.

## Setup Commands

- Build the project: `cargo build`
- Run the agent: `cargo run`
- Run tests: `cargo test`
- Run with verbose logging: `cargo run -- --verbose`

## Code Style

- Use Rust 2024 edition
- Follow standard Rust formatting with `cargo fmt`
- Use double quotes for strings when possible
- Prefer functional programming patterns
- Use `anyhow::Result` for error handling

## Testing Instructions

- Run all tests: `cargo test`
- Run specific test: `cargo test test_name`
- Run tests with output: `cargo test -- --nocapture`
- Integration tests are in the `tests/` directory

## Tool Usage Guidelines

When using the available tools:

1. **Web Search**: Use for finding current information, documentation, or examples
2. **File Operations**: Always explain what files you're reading/writing and why
3. **Code Search**: Use regex patterns to find specific code patterns or functions
4. **Shell Commands**: Be cautious with destructive operations, prefer read-only commands
5. **File Editing**: Make incremental changes and explain the reasoning

## Security Considerations

- Never execute commands that could harm the system
- Be careful with file permissions and ownership
- Validate user input before processing
- Use the YOLO mode flag only when explicitly requested

## Architecture Notes

- The agent uses SQLite for context storage with vector embeddings
- Multiple LLM providers are supported (OpenAI, OpenRouter, Ollama)
- Tools are implemented using the Rig framework's tool system
- Web interface is available for monitoring sessions and logs
