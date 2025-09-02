# Test Documentation

This document describes the comprehensive test suite for the Vega AI chat agent.

## Test Structure

### Unit Tests

#### 1. Provider Tests (`src/providers.rs`)

- **Provider Creation**: Tests for creating Ollama and OpenRouter providers
- **Error Handling**: Tests for invalid providers and missing API keys
- **Model Getter**: Tests for retrieving model names from providers

#### 2. Agent Base Tests (`src/agents/mod.rs`)

- **Config Creation**: Tests for creating agent configurations
- **Config Cloning**: Tests for cloning configurations
- **Field Validation**: Tests for proper field assignment

#### 3. Chat Agent Tests (`src/agents/chat.rs`)

- **Agent Creation**: Tests for creating chat agents with different providers
- **Error Scenarios**: Tests for invalid configurations
- **Config Preservation**: Tests that configurations are properly stored
- **Verbose Mode**: Tests for verbose logging configuration

#### 4. CLI Tests (`src/main.rs`)

- **Default Arguments**: Tests for default CLI argument values
- **Flag Parsing**: Tests for verbose, provider, model flags
- **Combined Arguments**: Tests for multiple arguments together
- **Config Integration**: Tests for converting CLI args to agent config

### Integration Tests (`tests/integration_tests.rs`)

- **End-to-End Agent Creation**: Tests the full flow from config to agent
- **Provider Integration**: Tests that different providers work with agents
- **Error Handling Integration**: Tests error scenarios across modules
- **Configuration Flow**: Tests that configurations flow properly through the system

## Test Coverage

The test suite covers:

✅ **Provider Logic**

- All supported providers (Ollama, OpenRouter)
- Error conditions (missing API keys, invalid providers)
- Model name retrieval

✅ **Agent Framework**

- Base agent trait functionality
- Configuration management
- Agent creation and initialization

✅ **Chat Agent**

- Creation with different providers
- Configuration validation
- Error handling

✅ **CLI Interface**

- Argument parsing
- Default values
- Flag combinations
- Environment variable support

✅ **Integration**

- Full system integration
- Cross-module interactions
- Error propagation

## Running Tests

```bash
# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test integration_tests

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_chat_agent_creation_with_ollama
```

## Test Statistics

- **Total Tests**: 25+ tests
- **Unit Tests**: 21 tests across 4 modules
- **Integration Tests**: 4 comprehensive integration tests
- **Coverage**: All public APIs and error conditions

## Future Test Additions

When adding new agent types, ensure to add:

1. **Unit tests** for the new agent's creation and configuration
2. **Integration tests** for the new agent with different providers
3. **Error handling tests** for invalid configurations
4. **CLI tests** if new command-line options are added

## Test Dependencies

- `tokio-test`: For async testing support
- `mockall`: For mocking (available but not currently used)
- Standard Rust testing framework
