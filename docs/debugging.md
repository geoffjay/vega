# Debugging Vega

This document explains how to use Vega's comprehensive debug logging to troubleshoot issues and understand the agent's processing pipeline.

## Debug Logging Levels

Vega uses the `tracing` crate for structured logging with the following levels:

- **ERROR**: Critical errors that prevent operation
- **WARN**: Warning conditions that don't prevent operation
- **INFO**: General information about operations (default level)
- **DEBUG**: Detailed information for debugging (enabled with `--verbose`)
- **TRACE**: Very detailed execution tracing (enabled with `RUST_LOG=trace`)

## Enabling Debug Logging

### Basic Debug Logging

Use the `--verbose` flag to enable debug-level logging:

```bash
cargo run -- --verbose
```

This will show:

- General debug information
- Tool execution details
- Context retrieval information
- LLM request/response status

### Trace Level Logging

For the most detailed logging, set the `VEGA_LOG_LEVEL` environment variable to `trace`:

```bash
VEGA_LOG_LEVEL=trace cargo run --bin vega -- --verbose --log-output console
```

You can also use the standard `RUST_LOG` environment variable:

```bash
RUST_LOG=trace cargo run --bin vega -- --verbose --log-output console
```

This will show:

- User prompt receipt
- Embedding generation progress
- Context retrieval details
- Full LLM request/response flow
- Tool execution confirmation and results
- Detailed error information

### Selective Logging

You can also enable trace logging for specific modules:

```bash
# Only trace logging for the chat agent
VEGA_LOG_LEVEL=vega::agents::chat=trace cargo run --bin vega -- --verbose --log-output console

# Trace logging for tools only
VEGA_LOG_LEVEL=vega::tools=trace cargo run --bin vega -- --verbose --log-output console

# Multiple modules
VEGA_LOG_LEVEL=vega::agents::chat=trace,vega::tools=trace cargo run --bin vega -- --verbose --log-output console
```

## Debug Output Examples

### Prompt Processing Pipeline

When you enter a prompt like "review this file", you'll see trace logs showing:

1. **Prompt Receipt**:

   ```
   TRACE vega::agents::chat: Received user prompt: 'review this file'
   ```

2. **Embedding Generation**:

   ```
   TRACE vega::agents::chat: Generating embedding for prompt...
   TRACE vega::agents::chat: Embedding generated successfully (dimension: 1536)
   ```

3. **Context Retrieval**:

   ```
   TRACE vega::agents::chat: Retrieving relevant context...
   TRACE vega::agents::chat: Retrieved 3 context entries
   ```

4. **LLM Request**:

   ```
   TRACE vega::agents::chat: Built full prompt for LLM (length: 1247 chars)
   TRACE vega::agents::chat: Sending request to LLM with tools...
   TRACE vega::agents::chat: Attempting LLM request with provider: openai
   TRACE vega::agents::chat: Creating OpenAI client and agent...
   TRACE vega::agents::chat: Building agent with model: gpt-4
   TRACE vega::agents::chat: Sending prompt to OpenAI agent...
   ```

5. **Tool Execution**:

   ```
   TRACE vega::tools::confirmed: Tool execution requested: read_file - Read file: ./test_debug.md
   TRACE vega::tools::confirmed: YOLO mode enabled, auto-confirming tool execution
   TRACE vega::tools::confirmed: Reading file: ./test_debug.md
   TRACE vega::tools::confirmed: File read completed successfully
   ```

6. **Response**:
   ```
   TRACE vega::agents::chat: OpenAI agent returned response (length: 342 chars)
   TRACE vega::agents::chat: LLM responded successfully
   ```

### Tool Confirmation Flow

When tools require user confirmation (non-YOLO mode):

```
TRACE vega::tools::confirmed: Tool execution requested: bash - Execute command: ls -la
🔧 Tool Execution Request:
Tool: bash
Action: Execute command: ls -la
Do you want to proceed? (y/N): y
TRACE vega::tools::confirmed: User response to tool confirmation: 'y' -> true
TRACE vega::tools::confirmed: Executing bash command: ls -la
TRACE vega::tools::confirmed: Bash command completed successfully
```

## Troubleshooting Common Issues

### Agent Hangs or No Response

If Vega appears to hang, enable trace logging to see where it stops:

```bash
VEGA_LOG_LEVEL=trace cargo run --bin vega -- --verbose --log-output console
```

Look for the last trace message to identify where the process is stuck:

- If it stops after "Sending prompt to OpenAI agent...", the issue is likely with the LLM API
- If it stops after "Tool execution requested...", the issue might be with tool confirmation
- If it stops during embedding generation, check your embedding service configuration

### Tool Execution Issues

Enable trace logging for tools to debug execution problems:

```bash
VEGA_LOG_LEVEL=vega::tools=trace cargo run --bin vega -- --verbose --log-output console
```

This will show:

- Tool confirmation requests and responses
- Actual tool execution attempts
- Success/failure status of tool operations

### LLM Communication Issues

For LLM-related problems, focus on the chat agent logs:

```bash
VEGA_LOG_LEVEL=vega::agents::chat=trace cargo run --bin vega -- --verbose --log-output console
```

This will show the full request/response cycle with the LLM provider.

## Performance Impact

Debug logging has minimal performance impact:

- **DEBUG level**: ~1-2% overhead
- **TRACE level**: ~3-5% overhead

The overhead is primarily from string formatting and I/O operations.

## Log Output Control

By default, logs go to the console. You can control this with the `--log-output` flag:

```bash
# Console only (default)
cargo run -- --log-output console

# File only
cargo run -- --log-output file

# Both console and file
cargo run -- --log-output console,file

# No console output (useful for trace logging)
cargo run -- --log-output file --verbose
```

## Best Practices

1. **Start with --verbose**: Use debug level first before moving to trace
2. **Use selective logging**: Target specific modules when debugging known issues
3. **Capture output**: Redirect trace logs to a file for analysis:
   ```bash
   VEGA_LOG_LEVEL=trace cargo run --bin vega -- --verbose --log-output console 2> debug.log
   ```
4. **Check timestamps**: Use log timestamps to identify performance bottlenecks
5. **Filter noise**: Use module-specific logging to reduce irrelevant output

## Integration with Development

The debug logging is designed to help with:

- **Development**: Understanding the agent's decision-making process
- **Debugging**: Identifying where issues occur in the pipeline
- **Performance**: Finding bottlenecks in processing
- **Testing**: Verifying that tools execute as expected
- **Monitoring**: Tracking agent behavior in production

The trace logs provide a complete audit trail of every operation Vega performs, making it easier to understand and debug complex interactions.
