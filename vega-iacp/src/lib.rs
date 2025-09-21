//! # Vega Inter-Agent Communication Protocol (IaCP)
//!
//! This crate provides the implementation of the Inter-Agent Communication Protocol
//! for the Vega AI agent framework, enabling distributed agents to collaborate
//! effectively through standardized communication patterns.
//!
//! ## Features
//!
//! - Human-readable JSON message format for transparency and debugging
//! - TCP/IP based reliable message transport
//! - Agent discovery and registration
//! - Task delegation and coordination
//! - Tool execution requests between agents
//! - Context and knowledge sharing
//!
//! ## Usage
//!
//! This crate is currently in initial development phase. Implementation
//! will follow the IaCP specification defined in the project documentation.

pub mod protocol {
    //! Core protocol definitions and message types

    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    /// IaCP protocol version
    pub const IACP_VERSION: &str = "1.0";

    /// Agent information for message routing
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AgentInfo {
        pub agent_id: String,
        pub agent_name: String,
        pub capabilities: Vec<String>,
    }

    /// Message recipient specification
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Recipient {
        pub agent_id: Option<String>,
        pub broadcast: bool,
    }

    /// Message metadata for processing hints
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MessageMetadata {
        pub priority: Priority,
        pub expires_at: Option<DateTime<Utc>>,
        pub requires_response: bool,
        pub response_timeout: Option<u32>,
    }

    /// Message priority levels
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum Priority {
        Low,
        Normal,
        High,
        Urgent,
    }

    impl Default for Priority {
        fn default() -> Self {
            Priority::Normal
        }
    }

    /// Base IaCP message structure
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct IacpMessage {
        pub iacp_version: String,
        pub message_id: Uuid,
        pub timestamp: DateTime<Utc>,
        pub sender: AgentInfo,
        pub recipient: Recipient,
        pub message_type: String,
        pub conversation_id: Option<Uuid>,
        pub parent_message_id: Option<Uuid>,
        pub payload: serde_json::Value,
        pub metadata: MessageMetadata,
    }

    impl IacpMessage {
        /// Create a new IaCP message
        pub fn new(
            sender: AgentInfo,
            recipient: Recipient,
            message_type: String,
            payload: serde_json::Value,
        ) -> Self {
            Self {
                iacp_version: IACP_VERSION.to_string(),
                message_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                sender,
                recipient,
                message_type,
                conversation_id: None,
                parent_message_id: None,
                payload,
                metadata: MessageMetadata {
                    priority: Priority::default(),
                    expires_at: None,
                    requires_response: false,
                    response_timeout: None,
                },
            }
        }

        /// Set conversation context
        pub fn with_conversation(
            mut self,
            conversation_id: Uuid,
            parent_message_id: Option<Uuid>,
        ) -> Self {
            self.conversation_id = Some(conversation_id);
            self.parent_message_id = parent_message_id;
            self
        }

        /// Set message metadata
        pub fn with_metadata(mut self, metadata: MessageMetadata) -> Self {
            self.metadata = metadata;
            self
        }

        /// Convert message to JSON bytes for transmission
        pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
            Ok(serde_json::to_vec(self)?)
        }

        /// Parse message from JSON bytes
        pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
            Ok(serde_json::from_slice(bytes)?)
        }
    }
}

pub mod network {
    //! Network transport layer for IaCP messages

    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tracing::{debug, info};

    /// TCP transport configuration
    #[derive(Debug, Clone)]
    pub struct TransportConfig {
        pub bind_address: String,
        pub port_range: (u16, u16),
        pub max_message_size: usize,
        pub connection_timeout: std::time::Duration,
        pub heartbeat_interval: std::time::Duration,
    }

    impl Default for TransportConfig {
        fn default() -> Self {
            Self {
                bind_address: "127.0.0.1".to_string(),
                port_range: (9000, 9999),
                max_message_size: 16 * 1024 * 1024, // 16MB
                connection_timeout: std::time::Duration::from_secs(30),
                heartbeat_interval: std::time::Duration::from_secs(30),
            }
        }
    }

    /// IaCP network transport
    pub struct IacpTransport {
        config: TransportConfig,
        _state: Arc<Mutex<()>>, // Placeholder for future connection state
    }

    impl IacpTransport {
        /// Create a new IaCP transport instance
        pub fn new(config: TransportConfig) -> Self {
            info!("Initializing IaCP transport with config: {:?}", config);
            Self {
                config,
                _state: Arc::new(Mutex::new(())),
            }
        }

        /// Start the transport server (placeholder)
        pub async fn start(&self) -> anyhow::Result<()> {
            debug!("Starting IaCP transport server");
            // TODO: Implement TCP server startup
            Ok(())
        }

        /// Stop the transport server (placeholder)
        pub async fn stop(&self) -> anyhow::Result<()> {
            debug!("Stopping IaCP transport server");
            // TODO: Implement graceful shutdown
            Ok(())
        }

        /// Get transport configuration
        pub fn config(&self) -> &TransportConfig {
            &self.config
        }
    }
}

pub mod agent {
    //! Agent management and discovery functionality

    use crate::protocol::AgentInfo;
    use std::collections::HashMap;
    use tokio::sync::RwLock;
    use tracing::{debug, info};

    /// Agent registry for discovery and routing
    pub struct AgentRegistry {
        agents: RwLock<HashMap<String, AgentInfo>>,
    }

    impl AgentRegistry {
        /// Create a new agent registry
        pub fn new() -> Self {
            Self {
                agents: RwLock::new(HashMap::new()),
            }
        }

        /// Register a new agent
        pub async fn register_agent(&self, agent: AgentInfo) -> anyhow::Result<()> {
            let mut agents = self.agents.write().await;
            info!(
                "Registering agent: {} ({})",
                agent.agent_name, agent.agent_id
            );
            agents.insert(agent.agent_id.clone(), agent);
            Ok(())
        }

        /// Unregister an agent
        pub async fn unregister_agent(&self, agent_id: &str) -> anyhow::Result<()> {
            let mut agents = self.agents.write().await;
            if agents.remove(agent_id).is_some() {
                info!("Unregistered agent: {}", agent_id);
            } else {
                debug!("Attempted to unregister unknown agent: {}", agent_id);
            }
            Ok(())
        }

        /// Find agents by capability
        pub async fn find_agents_by_capability(&self, capability: &str) -> Vec<AgentInfo> {
            let agents = self.agents.read().await;
            agents
                .values()
                .filter(|agent| agent.capabilities.contains(&capability.to_string()))
                .cloned()
                .collect()
        }

        /// Get all registered agents
        pub async fn get_all_agents(&self) -> Vec<AgentInfo> {
            let agents = self.agents.read().await;
            agents.values().cloned().collect()
        }

        /// Get specific agent by ID
        pub async fn get_agent(&self, agent_id: &str) -> Option<AgentInfo> {
            let agents = self.agents.read().await;
            agents.get(agent_id).cloned()
        }
    }

    impl Default for AgentRegistry {
        fn default() -> Self {
            Self::new()
        }
    }
}

pub mod error {
    //! Error types for IaCP operations

    use thiserror::Error;

    /// IaCP specific errors
    #[derive(Error, Debug)]
    pub enum IacpError {
        #[error("Protocol version mismatch: expected {expected}, got {actual}")]
        VersionMismatch { expected: String, actual: String },

        #[error("Message format invalid: {reason}")]
        InvalidMessageFormat { reason: String },

        #[error("Agent not found: {agent_id}")]
        AgentNotFound { agent_id: String },

        #[error("Network transport error: {source}")]
        NetworkError { source: anyhow::Error },

        #[error("Authentication failed: {reason}")]
        AuthenticationFailed { reason: String },

        #[error("Message timeout: waited {timeout_ms}ms")]
        MessageTimeout { timeout_ms: u64 },
    }
}

// Re-export commonly used types
pub use agent::AgentRegistry;
pub use error::IacpError;
pub use network::{IacpTransport, TransportConfig};
pub use protocol::{AgentInfo, IacpMessage, MessageMetadata, Priority, Recipient};
