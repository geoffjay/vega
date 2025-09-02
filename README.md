# Ally - AI Chat Agent

An AI chat agent built with Rust using the [Rig framework](https://github.com/0xPlaygrounds/rig), inspired by the Go implementation at [how-to-build-a-coding-agent](https://github.com/ghuntley/how-to-build-a-coding-agent).

## Features

- 🦀 **Rust-powered**: Built with Rust for performance and safety
- 🔌 **Multiple LLM Providers**: Support for Ollama and OpenRouter
- 💬 **Interactive Chat**: Command-line chat interface with colored output
- 🔧 **Configurable**: Flexible configuration via command-line arguments
- 📝 **Logging**: Optional verbose logging for debugging
- 🔗 **Agent Client Protocol (ACP)**: Compatible with ACP-enabled editors like Zed
- 🧠 **Context Awareness**: Persistent conversation history with embedding-based retrieval
- 🛠️ **Tool Support**: File operations, web search, code analysis, and more

## Prerequisites

### For Ollama (Local LLM)

- Install [Ollama](https://ollama.ai/)
- Pull a model (e.g., `ollama pull llama3.2`)
- Start Ollama service

### For OpenRouter (Cloud LLM)

- Get an API key from [OpenRouter](https://openrouter.ai/)
- Set the `OPENROUTER_API_KEY` environment variable or use the `--openrouter-api-key` flag

## Installation

1. Clone this repository:

   ```bash
   git clone <repository-url>
   cd ally
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

## Usage

### Basic Usage with Ollama (Default)

```bash
# Use default settings (Ollama with llama3.2 model)
cargo run

# Or with the built binary
./target/release/ally
```

### Using OpenRouter

```bash
# Set API key via environment variable
export OPENROUTER_API_KEY="your-api-key-here"
cargo run -- --provider openrouter --model "anthropic/claude-3.5-sonnet"

# Or pass API key directly
cargo run -- --provider openrouter --model "anthropic/claude-3.5-sonnet" --openrouter-api-key "your-api-key"
```

### Command Line Options

```bash
# Show help
cargo run -- --help

# Enable verbose logging
cargo run -- --verbose

# Use different model with Ollama
cargo run -- --model "llama2"

# Use different provider and model
cargo run -- --provider openrouter --model "openai/gpt-4"
```

### Agent Client Protocol (ACP) Mode

Ally supports the Agent Client Protocol, allowing it to be used as an AI coding agent in compatible editors like Zed:

```bash
# Run Ally in ACP mode
ally --acp

# ACP mode with specific provider and model
ally --acp --provider openrouter --model openai/gpt-4 --openrouter-api-key YOUR_KEY

# ACP mode with Ollama
ally --acp --provider ollama --model llama3.2
```

For detailed ACP integration information, see [ACP_INTEGRATION.md](ACP_INTEGRATION.md).

### Full Command Reference

```
Usage: ally [OPTIONS]

Options:
  -v, --verbose                        Enable verbose logging
  -p, --provider <PROVIDER>            LLM provider to use (ollama or openrouter) [default: ollama]
  -m, --model <MODEL>                  Model name to use [default: llama3.2]
      --openrouter-api-key <API_KEY>   OpenRouter API key (required if using openrouter provider)
                                       Can also be set via OPENROUTER_API_KEY environment variable
      --acp                            Run in Agent Client Protocol (ACP) mode for editor integration
  -h, --help                           Print help
  -V, --version                        Print version
```

## Chat Commands

Once the chat session starts:

- Type your message and press Enter to send
- Type `quit` or `exit` to end the session
- Use `Ctrl+C` to force quit

## Examples

### Example Chat Session

```
$ cargo run
Chat with AI Agent (use 'quit' or Ctrl+C to exit)
Type your message and press Enter to send.

You: Hello! Can you help me with Rust programming?
Agent: Hello! I'd be happy to help you with Rust programming. Rust is a systems programming language that focuses on safety, speed, and concurrency. What specific aspect of Rust would you like to learn about or get help with?

You: What are the main ownership rules in Rust?
Agent: Rust's ownership system has three main rules:

1. **Each value in Rust has a variable that's called its owner**
2. **There can only be one owner at a time**
3. **When the owner goes out of scope, the value will be dropped**

These rules help Rust manage memory safely without a garbage collector...

You: quit
```

### Using with Different Models

```bash
# Ollama with different models
cargo run -- --model "codellama"
cargo run -- --model "mistral"

# OpenRouter with various models
export OPENROUTER_API_KEY="your-key"
cargo run -- --provider openrouter --model "anthropic/claude-3.5-sonnet"
cargo run -- --provider openrouter --model "openai/gpt-4-turbo"
cargo run -- --provider openrouter --model "meta-llama/llama-3.1-8b-instruct"
```

## Development

### Building from Source

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Check code without building
cargo check
```

### Dependencies

- `rig-core`: LLM framework for Rust
- `tokio`: Async runtime
- `clap`: Command-line argument parsing
- `anyhow`: Error handling
- `tracing`: Logging framework
- `serde`: Serialization

## Architecture

The project is structured around:

- **`ChatAgent`**: Main agent struct that handles the chat loop
- **`LLMProvider`**: Enum supporting different LLM providers (Ollama, OpenRouter)
- **Provider-specific clients**: Abstracted through the Rig framework

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Inspired by the [Go chat agent implementation](https://github.com/ghuntley/how-to-build-a-coding-agent)
- Built with the [Rig framework](https://github.com/0xPlaygrounds/rig)
- Uses [Ollama](https://ollama.ai/) for local LLM support
- Uses [OpenRouter](https://openrouter.ai/) for cloud LLM access
