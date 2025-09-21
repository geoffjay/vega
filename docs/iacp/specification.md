# Inter-Agent Communication Protocol (IaCP) Specification

**Version:** 1.0-draft  
**Date:** September 21, 2025  
**Status:** Draft Specification

## 1. Overview and Objectives

### 1.1 Purpose

The Inter-Agent Communication Protocol (IaCP) enables distributed AI agents to collaborate effectively within a multi-agent framework. The protocol facilitates task delegation, information sharing, tool execution requests, and general agent-to-agent communication while maintaining human-readable transparency.

### 1.2 Design Principles

- **Human Readability**: All protocol messages must be human-readable for debugging, auditing, and traceability
- **Transparency**: Complete communication logs for accountability and system understanding
- **Extensibility**: Protocol must support future message types and capabilities
- **Reliability**: Built-in error handling and retry mechanisms
- **Simplicity**: Clear, straightforward message formats that agents can easily implement
- **Interoperability**: Language and platform-agnostic design

### 1.3 Core Capabilities

The protocol enables agents to:

- Initiate and delegate tasks to other agents
- Share contextual information and knowledge
- Request tool executions from specialized agents
- Broadcast announcements to agent networks
- Discover and register with other agents
- Coordinate complex multi-agent workflows

## 2. Message Format and Structure

### 2.1 Base Message Format

All IaCP messages use JSON format for human readability and language interoperability:

```json
{
  "iacp_version": "1.0",
  "message_id": "uuid-v4-string",
  "timestamp": "ISO-8601-timestamp",
  "sender": {
    "agent_id": "unique-agent-identifier",
    "agent_name": "human-readable-name",
    "capabilities": ["list", "of", "capabilities"]
  },
  "recipient": {
    "agent_id": "target-agent-id",
    "broadcast": false
  },
  "message_type": "message-type-enum",
  "conversation_id": "optional-conversation-uuid",
  "parent_message_id": "optional-parent-message-uuid",
  "payload": {
    // Message-type-specific content
  },
  "metadata": {
    "priority": "low|normal|high|urgent",
    "expires_at": "optional-iso-8601-timestamp",
    "requires_response": true,
    "response_timeout": 30
  }
}
```

### 2.2 Message Routing

- **Direct Messages**: Sent to specific agent via `recipient.agent_id`
- **Broadcast Messages**: Sent to all agents via `recipient.broadcast = true`
- **Group Messages**: Sent to agents with specific capabilities or roles

### 2.3 Message Lifecycle

1. **Generation**: Agent creates message with unique ID and timestamp
2. **Transmission**: Message sent via TCP connection
3. **Acknowledgment**: Recipient confirms receipt
4. **Processing**: Recipient processes message content
5. **Response**: Optional response message sent back
6. **Completion**: Conversation marked complete or continues

## 3. Communication Patterns

### 3.1 Request-Response Pattern

Used for direct agent interactions requiring a response:

```json
// Request
{
  "message_type": "task_request",
  "metadata": {"requires_response": true, "response_timeout": 60},
  "payload": {
    "task_type": "analysis",
    "description": "Analyze user sentiment in chat logs",
    "parameters": {...}
  }
}

// Response
{
  "message_type": "task_response",
  "parent_message_id": "original-request-id",
  "payload": {
    "status": "completed|failed|in_progress",
    "result": {...},
    "error": "optional-error-message"
  }
}
```

### 3.2 Fire-and-Forget Pattern

Used for information sharing without expecting response:

```json
{
  "message_type": "context_share",
  "metadata": {"requires_response": false},
  "payload": {
    "context_type": "user_preferences",
    "data": {...}
  }
}
```

### 3.3 Publish-Subscribe Pattern

Used for broadcast communications to interested agents:

```json
{
  "message_type": "event_notification",
  "recipient": {"broadcast": true},
  "payload": {
    "event_type": "user_session_started",
    "event_data": {...}
  }
}
```

## 4. Message Types

### 4.1 Task Management Messages

#### 4.1.1 Task Request (`task_request`)

```json
{
  "message_type": "task_request",
  "payload": {
    "task_type": "string",
    "description": "human-readable task description",
    "parameters": {
      "input_data": "...",
      "requirements": "...",
      "constraints": "..."
    },
    "priority": "low|normal|high|urgent",
    "deadline": "optional-iso-8601-timestamp"
  }
}
```

#### 4.1.2 Task Response (`task_response`)

```json
{
  "message_type": "task_response",
  "payload": {
    "status": "accepted|rejected|completed|failed|in_progress",
    "result": "task output data",
    "progress": 0.75,
    "estimated_completion": "optional-iso-8601-timestamp",
    "error": {
      "code": "error-code",
      "message": "error description"
    }
  }
}
```

### 4.2 Information Sharing Messages

#### 4.2.1 Context Share (`context_share`)

```json
{
  "message_type": "context_share",
  "payload": {
    "context_type": "user_preferences|session_data|knowledge|insights",
    "scope": "session|global|user-specific",
    "data": {
      // Context-specific information
    },
    "relevance_score": 0.85,
    "tags": ["tag1", "tag2"]
  }
}
```

#### 4.2.2 Knowledge Update (`knowledge_update`)

```json
{
  "message_type": "knowledge_update",
  "payload": {
    "update_type": "learned|corrected|deprecated",
    "knowledge_domain": "string",
    "content": {
      "facts": [...],
      "rules": [...],
      "patterns": [...]
    },
    "confidence": 0.9,
    "source": "user|inference|external"
  }
}
```

### 4.3 Tool Execution Messages

#### 4.3.1 Tool Request (`tool_request`)

```json
{
  "message_type": "tool_request",
  "payload": {
    "tool_name": "string",
    "parameters": {
      // Tool-specific parameters
    },
    "execution_context": {
      "user_id": "optional",
      "session_id": "optional",
      "permissions": ["read", "write"]
    },
    "callback_required": true
  }
}
```

#### 4.3.2 Tool Response (`tool_response`)

```json
{
  "message_type": "tool_response",
  "payload": {
    "status": "success|error|permission_denied",
    "output": "tool execution result",
    "execution_time": 1.23,
    "resources_used": {
      "cpu_time": 0.5,
      "memory": "10MB"
    },
    "error": {
      "code": "error-code",
      "message": "error description"
    }
  }
}
```

### 4.4 Network Management Messages

#### 4.4.1 Agent Registration (`agent_register`)

```json
{
  "message_type": "agent_register",
  "payload": {
    "agent_info": {
      "agent_id": "unique-identifier",
      "agent_name": "human-readable-name",
      "agent_type": "specialist|generalist|coordinator",
      "capabilities": ["capability1", "capability2"],
      "supported_tools": ["tool1", "tool2"],
      "version": "1.0.0",
      "endpoints": {
        "primary": "tcp://host:port",
        "health": "tcp://host:port/health"
      }
    },
    "network_info": {
      "max_connections": 100,
      "preferred_protocols": ["iacp/1.0"],
      "authentication_methods": ["token", "certificate"]
    }
  }
}
```

#### 4.4.2 Agent Discovery (`agent_discover`)

```json
{
  "message_type": "agent_discover",
  "payload": {
    "discovery_type": "all|by_capability|by_type",
    "criteria": {
      "capabilities": ["required", "capabilities"],
      "agent_type": "optional-agent-type",
      "load_threshold": 0.8
    },
    "response_format": "summary|detailed"
  }
}
```

### 4.5 Coordination Messages

#### 4.5.1 Workflow Initiation (`workflow_start`)

```json
{
  "message_type": "workflow_start",
  "payload": {
    "workflow_id": "unique-workflow-identifier",
    "workflow_name": "human-readable-name",
    "description": "workflow description",
    "participants": [
      {
        "agent_id": "agent-id",
        "role": "coordinator|worker|specialist",
        "responsibilities": ["task1", "task2"]
      }
    ],
    "execution_plan": {
      "steps": [...],
      "dependencies": [...],
      "timeouts": {...}
    }
  }
}
```

## 5. Network Protocol Details

### 5.1 TCP/IP Implementation

- **Transport**: TCP/IP for reliable, ordered delivery
- **Port Range**: Configurable, default range 9000-9999
- **Connection Model**: Persistent connections with keep-alive
- **Message Framing**: Length-prefixed messages for stream parsing

### 5.2 Message Framing Format

```
[4-byte length][JSON message payload]
```

- Length: 32-bit big-endian integer indicating payload size
- Payload: UTF-8 encoded JSON message
- Maximum message size: 16MB (configurable)

### 5.3 Connection Management

#### 5.3.1 Connection Establishment

1. TCP connection initiated
2. IaCP handshake with version negotiation
3. Authentication if required
4. Agent registration exchange

#### 5.3.2 Connection Maintenance

- Heartbeat messages every 30 seconds
- Connection timeout after 90 seconds of inactivity
- Automatic reconnection with exponential backoff

#### 5.3.3 Connection Termination

- Graceful shutdown with completion of pending messages
- Connection close notification to peers
- Resource cleanup

### 5.4 Error Handling

#### 5.4.1 Network Errors

- Connection failures: Automatic retry with backoff
- Timeout errors: Configurable retry attempts
- Malformed messages: Log and discard with error response

#### 5.4.2 Protocol Errors

```json
{
  "message_type": "error_response",
  "payload": {
    "error_code": "INVALID_MESSAGE_FORMAT|UNSUPPORTED_VERSION|AUTHENTICATION_FAILED",
    "error_message": "Human-readable error description",
    "details": {
      "expected": "what was expected",
      "received": "what was actually received"
    },
    "retry_allowed": true,
    "retry_after": 5
  }
}
```

## 6. Security Considerations

### 6.1 Authentication Methods

- **Token-based**: JWT tokens for agent authentication
- **Certificate-based**: X.509 certificates for mutual authentication
- **API Key**: Simple API key authentication for development

### 6.2 Authorization Model

- **Capability-based**: Agents authorized based on declared capabilities
- **Role-based**: Hierarchical permissions (admin, user, guest)
- **Resource-based**: Per-tool and per-data permissions

### 6.3 Message Security

- **Message Integrity**: Optional message signing with digital signatures
- **Confidentiality**: Optional TLS encryption for sensitive communications
- **Audit Trail**: Complete logging of all inter-agent communications

## 7. Implementation Guidelines

### 7.1 Agent Requirements

Every IaCP-compliant agent must:

- Implement core message types (task_request, task_response, error_response)
- Support agent registration and discovery
- Handle connection management properly
- Provide human-readable logging
- Implement proper error handling and recovery

### 7.2 Network Topology

- **Hub Model**: Central coordinator with spoke agents
- **Mesh Model**: Peer-to-peer agent connections
- **Hybrid Model**: Regional coordinators with local meshes

### 7.3 Performance Considerations

- **Message Batching**: Group multiple small messages for efficiency
- **Connection Pooling**: Reuse connections for multiple conversations
- **Load Balancing**: Distribute requests across capable agents
- **Circuit Breakers**: Prevent cascade failures in agent networks

## 8. Example Scenarios

### 8.1 Simple Task Delegation

Agent A requests sentiment analysis from Agent B:

```json
// Agent A -> Agent B
{
  "iacp_version": "1.0",
  "message_id": "123e4567-e89b-12d3-a456-426614174000",
  "timestamp": "2025-09-21T10:30:00Z",
  "sender": {"agent_id": "chat-agent-01", "agent_name": "Chat Handler"},
  "recipient": {"agent_id": "nlp-agent-01"},
  "message_type": "task_request",
  "payload": {
    "task_type": "sentiment_analysis",
    "description": "Analyze sentiment of user message",
    "parameters": {
      "text": "I love this new feature!",
      "language": "en"
    }
  },
  "metadata": {"requires_response": true, "response_timeout": 30}
}

// Agent B -> Agent A
{
  "iacp_version": "1.0",
  "message_id": "456e7890-e89b-12d3-a456-426614174001",
  "timestamp": "2025-09-21T10:30:02Z",
  "sender": {"agent_id": "nlp-agent-01", "agent_name": "NLP Processor"},
  "recipient": {"agent_id": "chat-agent-01"},
  "message_type": "task_response",
  "parent_message_id": "123e4567-e89b-12d3-a456-426614174000",
  "payload": {
    "status": "completed",
    "result": {
      "sentiment": "positive",
      "confidence": 0.95,
      "emotions": ["joy", "satisfaction"]
    }
  }
}
```

### 8.2 Multi-Agent Workflow

User requests comprehensive code analysis involving multiple specialized agents:

1. **Coordinator Agent** receives request and creates workflow
2. **Code Parser Agent** extracts structure and syntax
3. **Security Agent** performs security analysis
4. **Performance Agent** analyzes performance characteristics
5. **Documentation Agent** generates summary report
6. **Coordinator Agent** combines results and responds to user

## 9. Future Extensions

### 9.1 Planned Enhancements

- **Streaming Support**: Large data transfer with streaming protocol
- **Event Sourcing**: Complete audit trail with event replay capability
- **Load Balancing**: Intelligent request routing based on agent load
- **Monitoring**: Built-in metrics and health monitoring
- **Configuration Management**: Dynamic agent reconfiguration

### 9.2 Version Compatibility

- **Backward Compatibility**: New versions must support previous message formats
- **Feature Negotiation**: Agents negotiate supported features during handshake
- **Deprecation Policy**: 6-month notice for deprecated features

## 10. Appendices

### 10.1 Error Codes Reference

| Code                | Description                    | Retry | Action                    |
| ------------------- | ------------------------------ | ----- | ------------------------- |
| INVALID_FORMAT      | Message format invalid         | No    | Fix message format        |
| UNSUPPORTED_VERSION | Protocol version not supported | No    | Upgrade protocol          |
| AGENT_NOT_FOUND     | Target agent not available     | Yes   | Retry or find alternative |
| TIMEOUT             | Request timed out              | Yes   | Retry with longer timeout |
| PERMISSION_DENIED   | Insufficient permissions       | No    | Check authorization       |

### 10.1 Message Size Limits

| Component            | Limit         | Configurable |
| -------------------- | ------------- | ------------ |
| Maximum message size | 16MB          | Yes          |
| Agent ID length      | 64 characters | No           |
| Message ID format    | UUID v4       | No           |
| Payload nesting      | 10 levels     | Yes          |

---

**End of Specification**

This specification provides a comprehensive framework for inter-agent communication while maintaining the core principles of human readability, transparency, and extensibility. Implementation details and language-specific bindings will be developed in subsequent phases.
