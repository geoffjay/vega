//! # LLM Provider Implementations
//!
//! This module provides abstractions over different Large Language Model providers,
//! allowing the Vega agent to work with both local and cloud-based models.
//!
//! ## Supported Providers
//!
//! - **Ollama**: Local model execution with privacy and no API costs
//! - **OpenRouter**: Cloud-based access to multiple model providers
//! - **Anthropic**: Direct access to Claude models via Anthropic API
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use vega::providers::LLMProvider;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Local Ollama provider
//!     let ollama = LLMProvider::new("ollama", "llama3.1", None)?;
//!     
//!     // Cloud OpenRouter provider
//!     let openrouter = LLMProvider::new("openrouter", "openai/gpt-4", Some("api-key"))?;
//!     
//!     // Anthropic provider
//!     let anthropic = LLMProvider::new("anthropic", "claude-3-5-sonnet-20241022", Some("api-key"))?;
//!     
//!     // Send a prompt
//!     let response = ollama.prompt(
//!         "Hello, how are you?",
//!         "You are a helpful assistant.",
//!         1000
//!     ).await?;
//!     
//!     println!("Response: {}", response);
//!     Ok(())
//! }
//! ```

use anyhow::Result;
use rig::{client::CompletionClient, completion::Prompt, providers};
use std::fmt;

/// Enumeration of supported Large Language Model providers.
///
/// This enum abstracts over different LLM providers, allowing the application
/// to work with both local (Ollama) and cloud-based (OpenRouter, Anthropic) models
/// through a unified interface.
#[derive(Clone)]
pub enum LLMProvider {
    /// Ollama provider for local model execution.
    ///
    /// Provides access to locally hosted models through the Ollama service.
    /// Offers complete privacy as no data leaves the local machine.
    Ollama {
        /// The Ollama client instance
        client: providers::ollama::Client,
        /// The model name (e.g., "llama3.1", "codellama")
        model: String,
    },
    /// OpenRouter provider for cloud-based model access.
    ///
    /// Provides access to multiple model providers (OpenAI, Anthropic, etc.)
    /// through the OpenRouter API service.
    OpenRouter {
        /// The OpenRouter client instance
        client: providers::openrouter::Client,
        /// The model name (e.g., "openai/gpt-4", "anthropic/claude-3-sonnet")
        model: String,
    },
    /// Anthropic provider for direct Claude model access.
    ///
    /// Provides direct access to Claude models through the Anthropic API.
    /// Offers the latest Claude models with optimal performance.
    Anthropic {
        /// The Anthropic client instance
        client: providers::anthropic::Client,
        /// The model name (e.g., "claude-3-5-sonnet-20241022", "claude-3-haiku-20240307")
        model: String,
    },
}

impl LLMProvider {
    /// Creates a new LLM provider instance.
    ///
    /// # Arguments
    ///
    /// * `provider_name` - The name of the provider ("ollama", "openrouter", or "anthropic")
    /// * `model` - The model name to use
    /// * `api_key` - Optional API key (required for OpenRouter and Anthropic, ignored for Ollama)
    ///
    /// # Returns
    ///
    /// Returns a `Result<LLMProvider>` containing the configured provider instance.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The provider name is not supported
    /// - OpenRouter or Anthropic is specified but no API key is provided
    /// - The provider client cannot be initialized
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use vega::providers::LLMProvider;
    ///
    /// // Create Ollama provider (no API key needed)
    /// let ollama = LLMProvider::new("ollama", "llama3.1", None)?;
    ///
    /// // Create OpenRouter provider (API key required)
    /// let openrouter = LLMProvider::new("openrouter", "openai/gpt-4", Some("sk-..."))?;
    ///
    /// // Create Anthropic provider (API key required)
    /// let anthropic = LLMProvider::new("anthropic", "claude-3-5-sonnet-20241022", Some("sk-ant-..."))?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
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
            "anthropic" => {
                let api_key = api_key.ok_or_else(|| {
                    anyhow::anyhow!("Anthropic API key is required for anthropic provider. Set --anthropic-api-key or ANTHROPIC_API_KEY environment variable.")
                })?;

                let client = providers::anthropic::Client::new(api_key);
                Ok(LLMProvider::Anthropic {
                    client,
                    model: model.to_string(),
                })
            }
            _ => Err(anyhow::anyhow!(
                "Unsupported provider: {}. Supported providers: ollama, openrouter, anthropic",
                provider_name
            )),
        }
    }

    /// Returns the model name for this provider instance.
    ///
    /// # Returns
    ///
    /// A string slice containing the model name.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use vega::providers::LLMProvider;
    ///
    /// let provider = LLMProvider::new("ollama", "llama3.1", None)?;
    /// assert_eq!(provider.model(), "llama3.1");
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn model(&self) -> &str {
        match self {
            LLMProvider::Ollama { model, .. } => model,
            LLMProvider::OpenRouter { model, .. } => model,
            LLMProvider::Anthropic { model, .. } => model,
        }
    }

    /// Sends a prompt to the LLM and returns the response.
    ///
    /// This method handles the communication with the underlying LLM provider,
    /// whether it's a local Ollama instance or a cloud-based OpenRouter service.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The user prompt/message to send to the LLM
    /// * `preamble` - System instructions or context for the LLM
    /// * `max_tokens` - Maximum number of tokens in the response
    ///
    /// # Returns
    ///
    /// Returns a `Result<String>` containing the LLM's response text.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The network request fails (for cloud providers)
    /// - The LLM service is unavailable
    /// - The response cannot be parsed
    /// - Rate limits are exceeded
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use vega::providers::LLMProvider;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let provider = LLMProvider::new("ollama", "llama3.1", None)?;
    ///     
    ///     let response = provider.prompt(
    ///         "What is the capital of France?",
    ///         "You are a helpful geography assistant.",
    ///         100
    ///     ).await?;
    ///     
    ///     println!("Response: {}", response);
    ///     Ok(())
    /// }
    /// ```
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
            LLMProvider::Anthropic { client, model } => {
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
            LLMProvider::Anthropic { model, .. } => f
                .debug_struct("Anthropic")
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
    fn test_anthropic_provider_creation_with_api_key() {
        let provider = LLMProvider::new(
            "anthropic",
            "claude-3-5-sonnet-20241022",
            Some("test-api-key"),
        );
        assert!(provider.is_ok());

        if let Ok(LLMProvider::Anthropic { model, .. }) = provider {
            assert_eq!(model, "claude-3-5-sonnet-20241022");
        } else {
            panic!("Expected Anthropic provider");
        }
    }

    #[test]
    fn test_anthropic_provider_creation_without_api_key() {
        let provider = LLMProvider::new("anthropic", "claude-3-5-sonnet-20241022", None);
        assert!(provider.is_err());

        let error = provider.unwrap_err();
        assert!(error.to_string().contains("Anthropic API key is required"));
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

        let anthropic_provider =
            LLMProvider::new("anthropic", "claude-3-5-sonnet-20241022", Some("test-key")).unwrap();
        assert_eq!(anthropic_provider.model(), "claude-3-5-sonnet-20241022");
    }
}
