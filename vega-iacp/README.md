# vega-iacp

Inter-Agent Communication Protocol (IaCP) implementation for the Vega AI agent framework.

## Overview

This crate provides the core implementation of IaCP, enabling distributed AI agents to collaborate effectively through standardized communication patterns. The protocol is designed with human-readable transparency and TCP/IP reliability as core principles.

## Features

- **Human-readable JSON messaging** - All messages use JSON format for debugging and auditability
- **TCP/IP transport layer** - Reliable, ordered message delivery over standard networking
- **Agent discovery and registration** - Dynamic agent network formation and management
- **Task delegation and coordination** - Structured patterns for multi-agent workflows
- **Context and knowledge sharing** - Information exchange between specialized agents
- **Extensible message types** - Support for custom message formats and capabilities

## Architecture

The crate is organized into several key modules:

- `protocol` - Core message types and format definitions
- `network` - TCP transport implementation and connection management
- `agent` - Agent registry and discovery functionality
- `error` - IaCP-specific error types and handling

## Current Status

🚧 **Initial Development Phase**

This crate is currently in its initial setup phase with basic structure and placeholder implementations. The core protocol specification is defined in `/docs/iacp/specification.md`.

## Integration

The crate is integrated as a workspace member of the main Vega project and is available through:

```rust
use vega::iacp::{IacpMessage, AgentRegistry, TransportConfig};
```

## Development

### Building

```bash
# Build just this crate
cargo build -p vega-iacp

# Build the entire workspace
cargo build
```

### Testing

```bash
# Test the integration
cargo run --example iacp_basic
```

### Dependencies

- `serde` - JSON serialization/deserialization
- `tokio` - Async runtime and networking
- `uuid` - Message and conversation identifiers
- `chrono` - Timestamp handling
- `anyhow` - Error handling
- `thiserror` - Custom error types

## Future Development

Implementation roadmap includes:

1. **TCP Transport Layer** - Complete networking implementation
2. **Message Routing** - Intelligent message delivery and load balancing
3. **Security Layer** - Authentication, authorization, and encryption
4. **Performance Optimization** - Connection pooling, message batching
5. **Monitoring and Metrics** - Built-in observability features

## License

MIT License - See main project license for details.
