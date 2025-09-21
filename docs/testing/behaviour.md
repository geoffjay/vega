# Agent Behavior Testing Guide

This document provides comprehensive guidance on testing and verifying agent behaviors in Vega, with a focus on ensuring the agent properly utilizes "thinking" phases and other expected behaviors.

## Overview

Vega includes multiple approaches to verify agent behaviorts:

1. **Automated Unit Tests** - Fast, isolated tests for specific behaviors
2. **Integration Tests** - End-to-end tests with real components
3. **Manual Verification Tools** - Interactive tools for real-time behavior observation
4. **Streaming Progress Monitoring** - Real-time verification of agent processing phases

## Testing the "Thinking" Behavior

### Why Test Thinking Behavior?

The "thinking" phase is crucial for ensuring that:

- The agent takes appropriate time to process complex requests
- Users receive visual feedback during long operations
- The agent follows expected processing phases
- Complex prompts receive more consideration than simple ones

### Automated Testing

#### Unit Tests

Run the behavior tests to verify thinking phases:

```bash
# Run all behavior tests
cargo test --test behavior_tests

# Run specific thinking behavior test
cargo test test_agent_thinking_behavior

# Run with output to see detailed results
cargo test test_agent_thinking_behavior -- --nocapture
```

#### Integration Tests

Run streaming integration tests:

```bash
# Run streaming integration tests
cargo test --test streaming_integration_tests

# Test complex prompt thinking duration
cargo test test_complex_prompt_longer_thinking -- --nocapture

# Test phase sequence
cargo test test_streaming_phases_sequence -- --nocapture
```

### Manual Verification

#### Using the Behavior Verifier Tool

The `behavior-verifier` binary provides real-time behavior monitoring:

```bash
# Build the behavior verifier
cargo build --bin behavior-verifier

# Basic verification with default prompt
./target/debug/behavior-verifier

# Test with specific provider and model
./target/debug/behavior-verifier --provider ollama --model llama3.2

# Interactive mode for testing multiple prompts
./target/debug/behavior-verifier --interactive

# Test with custom prompt and timing expectations
./target/debug/behavior-verifier \
  --prompt "Explain quantum computing" \
  --min-thinking-ms 200 \
  --max-thinking-s 10

# Verbose output for debugging
./target/debug/behavior-verifier --verbose
```

#### Example Output

```
🔧 VEGA AGENT BEHAVIOR VERIFIER
═══════════════════════════════════════
Provider: ollama
Model: llama3.2
Session: behavior-test

🚀 Starting behavior verification...
📝 Prompt: "Explain the concept of artificial intelligence"
⏱️  Expected thinking time: 100ms - 30s

🔄     50ms ⚙️ Preparing
🔄    150ms 🔍 Generating embeddings
🔄    225ms 📚 Retrieving context
🔄    300ms 🧠 Thinking
🔄   1200ms ✨ Finalizing

🔍 BEHAVIOR VERIFICATION REPORT
═══════════════════════════════════════
📊 Overall Results:
  Total Duration: 1.25s
  Phases Detected: 5

🧠 Thinking Behavior:
  ✅ Thinking phase detected
  ⏱️  Thinking duration: 900ms
  ✅ Thinking duration is appropriate

📋 Phase Sequence:
  ✅ Phase sequence is correct

📝 Detailed Phase Timeline:
  1.     50ms ⚙️ Preparing
  2.    150ms 🔍 Generating embeddings
  3.    225ms 📚 Retrieving context
  4.    300ms 🧠 Thinking
  5.   1250ms ✨ Finalizing

🎯 Verification Status:
  ✅ ALL BEHAVIORS VERIFIED SUCCESSFULLY
═══════════════════════════════════════
```

## Testing Different Scenarios

### 1. Simple vs Complex Prompts

Test that complex prompts receive more thinking time:

```bash
# Simple prompt
./target/debug/behavior-verifier --prompt "Hello"

# Complex prompt
./target/debug/behavior-verifier --prompt "Analyze the philosophical implications of artificial intelligence on human consciousness"
```

### 2. Tool Usage Scenarios

Test prompts that should trigger tool usage:

```bash
# Should trigger web search
./target/debug/behavior-verifier --prompt "Search for the latest news about AI"

# Should trigger file operations
./target/debug/behavior-verifier --prompt "Read the README file and summarize it"

# Should trigger code search
./target/debug/behavior-verifier --prompt "Find all functions named 'main' in this project"
```

### 3. Different Providers

Test behavior consistency across providers:

```bash
# Test with Ollama
./target/debug/behavior-verifier --provider ollama --model llama3.2

# Test with OpenRouter (requires API key)
./target/debug/behavior-verifier --provider openrouter --model gpt-4 --api-key YOUR_KEY

# Test with Anthropic (requires API key)
./target/debug/behavior-verifier --provider anthropic --model claude-3-sonnet --api-key YOUR_KEY
```

## Expected Behaviors

### Phase Sequence

The agent should follow this sequence for most requests:

1. **Preparing** (⚙️) - Initial setup and validation
2. **Embedding** (🔍) - Generate embeddings for the prompt
3. **Context Retrieval** (📚) - Retrieve relevant conversation history
4. **Thinking** (🧠) - Process the request and formulate response
5. **Tool Execution** (🔧) - Execute any required tools (optional)
6. **Finalizing** (✨) - Complete the response

### Timing Expectations

- **Simple prompts**: 100ms - 2s thinking time
- **Complex prompts**: 500ms - 10s thinking time
- **Tool-heavy prompts**: Additional time for tool execution
- **Total duration**: Should be reasonable for the complexity

### Quality Indicators

✅ **Good Behavior:**

- Thinking phase is present
- Thinking duration is proportional to complexity
- Phase sequence is logical
- Total time is reasonable
- Visual feedback is provided throughout

❌ **Poor Behavior:**

- Missing thinking phase
- Instant responses to complex questions
- Incorrect phase sequence
- Excessive or insufficient thinking time
- No visual feedback during processing

## Continuous Integration

### Adding Behavior Tests to CI

Add these commands to your CI pipeline:

```yaml
# In your CI configuration
- name: Run behavior tests
  run: |
    cargo test --test behavior_tests
    cargo test --test streaming_integration_tests

- name: Build behavior verifier
  run: cargo build --bin behavior-verifier

- name: Quick behavior verification
  run: |
    ./target/debug/behavior-verifier \
      --prompt "Test prompt for CI" \
      --min-thinking-ms 50 \
      --max-thinking-s 5
```

## Debugging Behavior Issues

### Common Issues and Solutions

1. **Missing Thinking Phase**

   - Check that `StreamingProgress` is properly initialized
   - Verify `update_phase(ProgressPhase::Thinking)` is called
   - Ensure progress indicators are not being skipped

2. **Inappropriate Thinking Duration**

   - Review thinking time calculation logic
   - Check for blocking operations during thinking phase
   - Verify async operations are properly awaited

3. **Incorrect Phase Sequence**
   - Trace through the agent's execution flow
   - Check for early returns or error conditions
   - Verify all phases are properly sequenced

### Debug Mode

Run with verbose output to see detailed execution:

```bash
./target/debug/behavior-verifier --verbose --prompt "Debug this behavior"
```

## Custom Behavior Tests

### Creating New Tests

To add new behavior tests:

1. **Add to `behavior_tests.rs`:**

```rust
#[tokio::test]
async fn test_my_custom_behavior() -> anyhow::Result<()> {
    // Your test implementation
    Ok(())
}
```

2. **Add to `streaming_integration_tests.rs`:**

```rust
#[tokio::test]
async fn test_my_integration_behavior() -> anyhow::Result<()> {
    // Your integration test
    Ok(())
}
```

3. **Test with behavior verifier:**

```bash
./target/debug/behavior-verifier --prompt "Your test prompt"
```

### Test Categories

- **Timing Tests**: Verify appropriate durations
- **Sequence Tests**: Verify correct phase ordering
- **Complexity Tests**: Verify behavior scales with complexity
- **Provider Tests**: Verify consistency across providers
- **Tool Tests**: Verify tool integration behavior
- **Error Tests**: Verify behavior during error conditions

## Best Practices

1. **Test Early and Often**: Run behavior tests during development
2. **Use Real Scenarios**: Test with realistic prompts and use cases
3. **Monitor Regressions**: Include behavior tests in CI/CD
4. **Document Expected Behavior**: Clearly define what constitutes good behavior
5. **Test Edge Cases**: Include error conditions and unusual inputs
6. **Verify User Experience**: Ensure visual feedback is appropriate

## Troubleshooting

### Test Failures

If behavior tests fail:

1. Check the specific failure message
2. Run with `--nocapture` to see detailed output
3. Use the behavior verifier for manual inspection
4. Review recent changes to agent logic
5. Verify provider connectivity and configuration

### Performance Issues

If thinking times are inappropriate:

1. Profile the actual LLM request duration
2. Check for blocking operations
3. Verify async/await usage
4. Review complexity calculation logic
5. Test with different providers/models

## Future Enhancements

Potential improvements to behavior testing:

- **Real-time Monitoring**: Dashboard for behavior metrics
- **Automated Benchmarking**: Performance regression detection
- **Behavior Profiles**: Different expectations per provider/model
- **User Feedback Integration**: Incorporate user satisfaction metrics
- **Advanced Analytics**: ML-based behavior analysis

---

This testing framework ensures that Vega agents behave predictably and provide appropriate user feedback during all operations, especially the crucial "thinking" phases that indicate active processing.
