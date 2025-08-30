# AGENTS Guidelines for Ally

This repository contains **Ally**, a Rust-based AI agent built with the Rig framework that supports multiple LLM providers (Ollama and OpenRouter). When working on this project interactively with an agent (e.g. Cursor AI), please follow the guidelines below to ensure smooth development.

## 1. Development Commands

### Primary Development

- **Always use `cargo run`** for development and testing agents
- **Use `cargo check`** for quick syntax and type checking without building
- **Use `cargo test`** to run the comprehensive test suite

### Testing Different Configurations

```bash
# Test with default Ollama setup
cargo run

# Test with OpenRouter (requires API key)
export OPENROUTER_API_KEY="your-key"
cargo run -- --provider openrouter --model "anthropic/claude-3.5-sonnet"

# Test with verbose logging
cargo run -- --verbose

# Test with different Ollama models
cargo run -- --model "codellama"

# Test different agent types (when available)
cargo run -- --agent-type chat
cargo run -- --agent-type code-analysis
```

## 2. LLM Provider Configuration

### Ollama (Default)

- Ensure Ollama is installed and running
- Pull required models: `ollama pull llama3.2`
- No API key required

### OpenRouter

- Obtain API key from [OpenRouter](https://openrouter.ai/)
- Set `OPENROUTER_API_KEY` environment variable
- Or use `--openrouter-api-key` flag

## 3. Testing Guidelines

This project has a comprehensive test suite covering:

### Unit Tests (21+ tests)

```bash
# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run specific module tests
cargo test providers
cargo test agents
```

### Integration Tests

```bash
# Run integration tests
cargo test --test integration_tests

# Run with output for debugging
cargo test -- --nocapture
```

### Test Coverage Areas

- ✅ Provider logic (Ollama, OpenRouter)
- ✅ Agent framework and configuration
- ✅ CLI argument parsing
- ✅ Error handling scenarios
- ✅ End-to-end integration
- ✅ Agent type implementations (ChatAgent, future agent types)

## 4. Code Structure and Conventions

### Project Architecture

- **`src/main.rs`**: CLI interface and application entry point
- **`src/agents/`**: Agent implementations (ChatAgent, future agent types, base traits)
- **`src/providers.rs`**: LLM provider abstractions
- **`tests/`**: Comprehensive test suite

### Agent Types

Currently implemented:

- **ChatAgent**: Interactive conversational agent

Planned/Future agent types:

- **CodeAnalysisAgent**: Code review and analysis
- **DocumentationAgent**: Documentation generation and summarization
- **TaskAgent**: Task planning and execution
- **ResearchAgent**: Information gathering and synthesis

When adding new agent types, follow the existing `ChatAgent` pattern in `src/agents/`.

### Coding Standards

- **Language**: Rust (`.rs`) for all new components
- **Async**: Use `tokio` for async operations
- **Error Handling**: Use `anyhow::Result` for error propagation
- **Logging**: Use `tracing` framework with appropriate levels
- **CLI**: Use `clap` with derive macros for argument parsing

### Module Organization

- Prefer modules for new functionality
- Keep provider-specific code in `providers.rs`
- Add new agent types to `agents/` directory following the existing patterns
- Maintain clear separation between CLI, agents, and providers

## 5. Git Conventions

This project follows [Conventional Commits](https://www.conventionalcommits.org/). See the `git-conventions` rule for detailed guidelines:

```bash
# Good commit examples
feat(agents): add new agent type for code analysis
feat(agents): implement document summarization agent
fix(providers): handle OpenRouter API timeout
docs(readme): update installation instructions
test(integration): add end-to-end provider tests
```

## 6. Dependency Management

When adding dependencies:

1. **Update `Cargo.toml`** with appropriate version constraints
2. **Run `cargo check`** to verify compatibility
3. **Update tests** if new functionality is added
4. **Document** any new environment variables or configuration
5. **Consider agent-specific needs** (e.g., file I/O for DocumentationAgent, HTTP clients for ResearchAgent)

### Key Dependencies

- `rig-core`: LLM framework (keep updated)
- `tokio`: Async runtime
- `clap`: CLI parsing
- `anyhow`: Error handling
- `tracing`: Logging

## 7. Useful Commands Reference

| Command                  | Purpose                                           |
| ------------------------ | ------------------------------------------------- |
| `cargo run`              | Start the ally agent (default: Ollama + llama3.2) |
| `cargo run -- --help`    | Show all CLI options                              |
| `cargo test`             | Run complete test suite                           |
| `cargo check`            | Quick syntax/type checking                        |
| `cargo build --release`  | Build optimized binary                            |
| `cargo run -- --verbose` | Run with debug logging                            |

### Development Workflow

```bash
# 1. Check current status
cargo check

# 2. Run tests
cargo test

# 3. Test the application
cargo run -- --verbose

# 4. Test with different providers
export OPENROUTER_API_KEY="your-key"
cargo run -- --provider openrouter --model "gpt-4"

# 5. Test different agent types (when available)
cargo run -- --agent-type chat
cargo run -- --agent-type code-analysis
```

---

Following these practices ensures efficient development of Ally AI agents while maintaining code quality and test coverage.
