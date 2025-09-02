//! # Vega - AI Chat Agent Library
//!
//! Vega is a Rust-based AI chat agent built using the Rig framework. It provides
//! a comprehensive set of tools for interacting with various LLM providers,
//! managing conversation context, and performing system operations.
//!
//! ## Features
//!
//! - **Multiple LLM Providers**: Support for Ollama (local) and OpenRouter (cloud) providers
//! - **Tool System**: Comprehensive set of tools for file operations, web search, code analysis
//! - **Context Management**: Persistent conversation history with embedding-based retrieval
//! - **Agent Instructions**: Flexible instruction system using AGENTS.md and VEGA.md files
//! - **Web Interface**: Optional web interface for session management and monitoring
//! - **ACP Support**: Compatible with Agent Client Protocol for editor integration
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use vega::{Agent, AgentConfig, LLMProvider};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create a provider (Ollama local or OpenRouter cloud)
//!     let provider = LLMProvider::new("ollama", "llama3.1", None)?;
//!     
//!     // Configure the agent
//!     let config = AgentConfig::default();
//!     
//!     // Create and run the agent
//!     let mut agent = Agent::new(config, provider).await?;
//!     agent.run().await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Module Overview
//!
//! - [`acp`] - Agent Client Protocol implementation for editor integration
//! - [`agent_instructions`] - System for loading and managing agent instructions
//! - [`agents`] - Core agent implementation and configuration
//! - [`context`] - Conversation context management and persistence
//! - [`embeddings`] - Vector embeddings for semantic search and context retrieval
//! - [`input`] - User input handling and processing
//! - [`logging`] - Structured logging system with multiple output targets
//! - [`providers`] - LLM provider implementations (Ollama, OpenRouter)
//! - [`tools`] - Tool system for file operations, web search, and system interaction
//! - [`web`] - Web interface for session management and monitoring

pub mod acp;
pub mod agent_instructions;
pub mod agents;
pub mod context;
pub mod embeddings;
pub mod input;
pub mod logging;
pub mod providers;
pub mod tools;
pub mod web;

// Re-export commonly used types for convenience
pub use agents::{Agent, AgentConfig};
pub use providers::LLMProvider;
pub use tools::*;
