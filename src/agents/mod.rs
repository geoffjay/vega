use anyhow::Result;
use async_trait::async_trait;

pub mod chat;

use crate::context::ContextStore;

/// Base trait for all agent types
#[async_trait]
pub trait Agent {
    /// Run the agent's main functionality with context support
    /// Returns Ok(Some(new_session_id)) if session should be switched
    /// Returns Ok(None) if agent should exit normally
    async fn run(&self, context: &ContextStore, session_id: &str) -> Result<Option<String>>;

    /// Get the agent's name/type
    fn name(&self) -> &'static str;

    /// Get the initial greeting question for this agent
    fn greeting(&self) -> &'static str;
}

/// Common configuration shared across agents
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub verbose: bool,
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub embedding_provider: String,
    pub embedding_model: Option<String>,
    pub openai_api_key: Option<String>,
}

impl AgentConfig {
    pub fn new(
        verbose: bool,
        provider: String,
        model: String,
        api_key: Option<String>,
        embedding_provider: String,
        embedding_model: Option<String>,
        openai_api_key: Option<String>,
    ) -> Self {
        Self {
            verbose,
            provider,
            model,
            api_key,
            embedding_provider,
            embedding_model,
            openai_api_key,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_creation() {
        let config = AgentConfig::new(
            true,
            "ollama".to_string(),
            "llama3.2".to_string(),
            None,
            "simple".to_string(),
            None,
            None,
        );

        assert_eq!(config.verbose, true);
        assert_eq!(config.provider, "ollama");
        assert_eq!(config.model, "llama3.2");
        assert_eq!(config.api_key, None);
        assert_eq!(config.embedding_provider, "simple");
        assert_eq!(config.embedding_model, None);
        assert_eq!(config.openai_api_key, None);
    }

    #[test]
    fn test_agent_config_with_api_key() {
        let config = AgentConfig::new(
            false,
            "openrouter".to_string(),
            "gpt-4".to_string(),
            Some("test-api-key".to_string()),
            "openai".to_string(),
            Some("text-embedding-3-small".to_string()),
            Some("openai-key".to_string()),
        );

        assert_eq!(config.verbose, false);
        assert_eq!(config.provider, "openrouter");
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.api_key, Some("test-api-key".to_string()));
        assert_eq!(config.embedding_provider, "openai");
        assert_eq!(
            config.embedding_model,
            Some("text-embedding-3-small".to_string())
        );
        assert_eq!(config.openai_api_key, Some("openai-key".to_string()));
    }

    #[test]
    fn test_agent_config_clone() {
        let config = AgentConfig::new(
            true,
            "ollama".to_string(),
            "llama3.2".to_string(),
            None,
            "simple".to_string(),
            None,
            None,
        );

        let cloned_config = config.clone();
        assert_eq!(config.verbose, cloned_config.verbose);
        assert_eq!(config.provider, cloned_config.provider);
        assert_eq!(config.model, cloned_config.model);
        assert_eq!(config.api_key, cloned_config.api_key);
        assert_eq!(config.embedding_provider, cloned_config.embedding_provider);
        assert_eq!(config.embedding_model, cloned_config.embedding_model);
        assert_eq!(config.openai_api_key, cloned_config.openai_api_key);
    }
}
