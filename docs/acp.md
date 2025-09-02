# Agent Client Protocol (ACP) Integration

Ally now supports the Agent Client Protocol (ACP), allowing it to be used as an AI coding agent in ACP-compatible editors like Zed.

## What is ACP?

The Agent Client Protocol standardizes communication between code editors and AI coding agents. This allows any ACP-compliant agent to work with any ACP-compliant editor, reducing integration overhead and promoting interoperability.

## Using Ally with ACP

### Running Ally in ACP Mode

To run Ally in ACP mode, use the `--acp` flag:

```bash
ally --acp
```

This will start Ally as an ACP server that communicates over standard input/output (stdio) using JSON-RPC.

### Configuration

All the standard Ally configuration options are available in ACP mode:

```bash
# Use with OpenRouter
ally --acp --provider openrouter --model openai/gpt-4 --openrouter-api-key YOUR_KEY

# Use with Ollama
ally --acp --provider ollama --model llama3.2

# Enable verbose logging
ally --acp --verbose

# Use custom context database
ally --acp --context-db /path/to/your/context.db
```

### Supported Features

The ACP integration currently supports:

- **Text-based conversations**: Full chat functionality with context awareness
- **File operations**: Reading and writing text files
- **Session management**: Multiple conversation sessions
- **Context persistence**: Conversation history stored in SQLite database
- **Embedding-based context retrieval**: Relevant context from previous conversations

### Unsupported Features (Future Roadmap)

- **Terminal operations**: Not yet implemented
- **Image/Audio content**: Text-only for now
- **MCP server integration**: Planned for future releases
- **Permission requests**: Currently denied by default

## Editor Integration

### Zed Editor

Ally can be used with Zed editor through the ACP integration. Configure Zed to use Ally as an agent:

1. Build Ally: `cargo build --release`
2. Configure Zed to use the Ally binary with the `--acp` flag
3. Set your preferred provider and model options

### Other ACP-Compatible Editors

Any editor that supports the Agent Client Protocol can use Ally. Refer to your editor's documentation for configuring ACP agents.

## Development

### Architecture

The ACP integration consists of:

- `AllyAcpAgent`: Implements the `acp::Agent` trait for handling agent-side operations
- `AllyAcpClient`: Implements the `acp::Client` trait for handling client-side operations
- Session management with atomic counters for thread safety
- Integration with Ally's existing chat agent and context system

### Key Files

- `src/acp.rs`: Main ACP implementation
- `src/main.rs`: Command-line integration with `--acp` flag
- `Cargo.toml`: Added `agent-client-protocol` dependency

### Testing

To test the ACP integration:

```bash
# Build the project
cargo build

# Test ACP mode (will wait for JSON-RPC input)
./target/debug/ally --acp

# Test with example client (if available)
cargo run --example client -- ./target/debug/ally --acp
```

### Protocol Compliance

The implementation follows the ACP specification:

- JSON-RPC 2.0 over stdio
- Standard ACP message types and error codes
- Proper session lifecycle management
- File operation support with path resolution

## Troubleshooting

### Common Issues

1. **"No such file or directory" errors**: Ensure file paths are correct and accessible
2. **Permission denied**: Check file permissions for read/write operations
3. **Model not found**: Verify your LLM provider configuration
4. **Context database errors**: Ensure the database path is writable

### Debugging

Enable verbose logging to see detailed ACP operations:

```bash
ally --acp --verbose
```

### Logs

ACP operations are logged through Ally's standard logging system. Check logs for:

- Session creation and management
- File operation requests
- Protocol message handling
- Error conditions

## Contributing

To contribute to the ACP integration:

1. Follow the existing code patterns in `src/acp.rs`
2. Add tests for new functionality
3. Update this documentation for new features
4. Ensure compatibility with the ACP specification

## Resources

- [Agent Client Protocol Documentation](https://agentclientprotocol.com/)
- [ACP Rust Crate Documentation](https://docs.rs/agent-client-protocol/)
- [Zed Editor ACP Integration](https://zed.dev/)
