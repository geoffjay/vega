use anyhow::Result;
use rig::{client::CompletionClient, completion::Prompt, providers};
use std::fmt;

/// Enumeration of supported LLM providers
#[derive(Clone)]
pub enum LLMProvider {
    Ollama {
        client: providers::ollama::Client,
        model: String,
    },
    OpenRouter {
        client: providers::openrouter::Client,
        model: String,
    },
}

impl LLMProvider {
    /// Create a new LLM provider instance
    pub fn new(provider_name: &str, model: &str, api_key: Option<&str>) -> Result<Self> {
        match provider_name {
            "ollama" => {
                let client = providers::ollama::Client::new();
                Ok(LLMProvider::Ollama {
                    client,
                    model: model.to_string(),
                })
            }
            "openrouter" => {
                let api_key = api_key.ok_or_else(|| {
                    anyhow::anyhow!("OpenRouter API key is required for openrouter provider. Set --openrouter-api-key or OPENROUTER_API_KEY environment variable.")
                })?;

                let client = providers::openrouter::Client::new(api_key);
                Ok(LLMProvider::OpenRouter {
                    client,
                    model: model.to_string(),
                })
            }
            _ => Err(anyhow::anyhow!(
                "Unsupported provider: {}. Supported providers: ollama, openrouter",
                provider_name
            )),
        }
    }

    /// Get the model name for this provider
    pub fn model(&self) -> &str {
        match self {
            LLMProvider::Ollama { model, .. } => model,
            LLMProvider::OpenRouter { model, .. } => model,
        }
    }

    /// Send a prompt to the LLM and get a response
    pub async fn prompt(&self, prompt: &str, preamble: &str, max_tokens: u64) -> Result<String> {
        let response = match self {
            LLMProvider::Ollama { client, model } => {
                let agent = client
                    .agent(model)
                    .preamble(preamble)
                    .max_tokens(max_tokens)
                    .build();
                agent.prompt(prompt).await?
            }
            LLMProvider::OpenRouter { client, model } => {
                let agent = client
                    .agent(model)
                    .preamble(preamble)
                    .max_tokens(max_tokens)
                    .build();
                agent.prompt(prompt).await?
            }
        };

        Ok(response)
    }
}

impl fmt::Debug for LLMProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LLMProvider::Ollama { model, .. } => f
                .debug_struct("Ollama")
                .field("model", model)
                .finish_non_exhaustive(),
            LLMProvider::OpenRouter { model, .. } => f
                .debug_struct("OpenRouter")
                .field("model", model)
                .finish_non_exhaustive(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_provider_creation() {
        let provider = LLMProvider::new("ollama", "llama3.2", None);
        assert!(provider.is_ok());

        if let Ok(LLMProvider::Ollama { model, .. }) = provider {
            assert_eq!(model, "llama3.2");
        } else {
            panic!("Expected Ollama provider");
        }
    }

    #[test]
    fn test_openrouter_provider_creation_with_api_key() {
        let provider = LLMProvider::new("openrouter", "gpt-4", Some("test-api-key"));
        assert!(provider.is_ok());

        if let Ok(LLMProvider::OpenRouter { model, .. }) = provider {
            assert_eq!(model, "gpt-4");
        } else {
            panic!("Expected OpenRouter provider");
        }
    }

    #[test]
    fn test_openrouter_provider_creation_without_api_key() {
        let provider = LLMProvider::new("openrouter", "gpt-4", None);
        assert!(provider.is_err());

        let error = provider.unwrap_err();
        assert!(error.to_string().contains("OpenRouter API key is required"));
    }

    #[test]
    fn test_unsupported_provider() {
        let provider = LLMProvider::new("unsupported", "model", None);
        assert!(provider.is_err());

        let error = provider.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Unsupported provider: unsupported")
        );
    }

    #[test]
    fn test_model_getter() {
        let ollama_provider = LLMProvider::new("ollama", "llama3.2", None).unwrap();
        assert_eq!(ollama_provider.model(), "llama3.2");

        let openrouter_provider =
            LLMProvider::new("openrouter", "gpt-4", Some("test-key")).unwrap();
        assert_eq!(openrouter_provider.model(), "gpt-4");
    }
}
