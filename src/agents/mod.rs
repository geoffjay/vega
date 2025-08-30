use anyhow::Result;
use async_trait::async_trait;

pub mod chat;

use crate::context::ContextStore;

/// Base trait for all agent types
#[async_trait]
pub trait Agent {
    /// Run the agent's main functionality with context support
    async fn run(&self, context: &ContextStore, session_id: &str) -> Result<()>;

    /// Get the agent's name/type
    fn name(&self) -> &'static str;
}

/// Common configuration shared across agents
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub verbose: bool,
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
}

impl AgentConfig {
    pub fn new(verbose: bool, provider: String, model: String, api_key: Option<String>) -> Self {
        Self {
            verbose,
            provider,
            model,
            api_key,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_creation() {
        let config = AgentConfig::new(true, "ollama".to_string(), "llama3.2".to_string(), None);

        assert_eq!(config.verbose, true);
        assert_eq!(config.provider, "ollama");
        assert_eq!(config.model, "llama3.2");
        assert_eq!(config.api_key, None);
    }

    #[test]
    fn test_agent_config_with_api_key() {
        let config = AgentConfig::new(
            false,
            "openrouter".to_string(),
            "gpt-4".to_string(),
            Some("test-api-key".to_string()),
        );

        assert_eq!(config.verbose, false);
        assert_eq!(config.provider, "openrouter");
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.api_key, Some("test-api-key".to_string()));
    }

    #[test]
    fn test_agent_config_clone() {
        let config = AgentConfig::new(true, "ollama".to_string(), "llama3.2".to_string(), None);

        let cloned_config = config.clone();
        assert_eq!(config.verbose, cloned_config.verbose);
        assert_eq!(config.provider, cloned_config.provider);
        assert_eq!(config.model, cloned_config.model);
        assert_eq!(config.api_key, cloned_config.api_key);
    }
}
