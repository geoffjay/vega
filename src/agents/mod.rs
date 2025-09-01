use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use minijinja::{Environment, UndefinedBehavior};
use std::collections::HashMap;
use std::env;

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

    /// Get the system prompt for this agent (defaults to empty string)
    fn system_prompt(&self) -> &str {
        ""
    }

    /// Render the system prompt with template variables
    fn render_system_prompt(&self) -> Result<String> {
        let template = self.system_prompt();
        if template.is_empty() {
            return Ok(String::new());
        }

        render_prompt_template(template)
    }
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
    pub yolo: bool,
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
        yolo: bool,
    ) -> Self {
        Self {
            verbose,
            provider,
            model,
            api_key,
            embedding_provider,
            embedding_model,
            openai_api_key,
            yolo,
        }
    }
}

/// Render a prompt template with supported variables
pub fn render_prompt_template(template: &str) -> Result<String> {
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);

    // Create context with supported variables
    let mut context = HashMap::new();

    // Add currentDateTime variable
    let current_time = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    context.insert("currentDateTime", current_time);

    // Add currentWorkingDirectory variable
    let current_dir = env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    context.insert("currentWorkingDirectory", current_dir);

    // Try to render the template
    match env.render_str(template, &context) {
        Ok(rendered) => Ok(rendered),
        Err(e) => {
            let error_msg = e.to_string();

            // Check if it's an undefined variable error
            if error_msg.contains("undefined value") {
                Err(anyhow::anyhow!(
                    "Unknown template variable. Supported variables are: currentDateTime, currentWorkingDirectory"
                ))
            } else {
                Err(anyhow::anyhow!("Template rendering error: {}", e))
            }
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
            false,
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
            false,
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
            false,
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

    #[test]
    fn test_render_prompt_template_empty() {
        let result = render_prompt_template("");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }

    #[test]
    fn test_render_prompt_template_no_variables() {
        let template = "Hello, this is a simple template without variables.";
        let result = render_prompt_template(template);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), template);
    }

    #[test]
    fn test_render_prompt_template_with_current_datetime() {
        let template = "The current date and time is {{currentDateTime}}.";
        let result = render_prompt_template(template);
        assert!(result.is_ok());
        let rendered = result.unwrap();
        assert!(rendered.starts_with("The current date and time is "));
        assert!(rendered.contains("UTC"));
    }

    #[test]
    fn test_render_prompt_template_with_current_working_directory() {
        let template = "Current working directory: {{currentWorkingDirectory}}";
        let result = render_prompt_template(template);
        assert!(result.is_ok());
        let rendered = result.unwrap();
        assert!(rendered.starts_with("Current working directory: "));
    }

    #[test]
    fn test_render_prompt_template_unknown_variable() {
        let template = "Hello {{unknownVariable}}!";
        let result = render_prompt_template(template);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Unknown template variable"));
        assert!(error_msg.contains("currentDateTime, currentWorkingDirectory"));
    }

    #[test]
    fn test_render_prompt_template_multiple_variables() {
        let template = "Time: {{currentDateTime}}, Directory: {{currentWorkingDirectory}}";
        let result = render_prompt_template(template);
        assert!(result.is_ok());
        let rendered = result.unwrap();
        assert!(rendered.contains("Time: "));
        assert!(rendered.contains("Directory: "));
        assert!(rendered.contains("UTC"));
    }
}
