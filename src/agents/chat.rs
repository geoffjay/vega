use anyhow::Result;
use async_trait::async_trait;
use std::io::{self, Write};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::{Agent, AgentConfig};
use crate::context::{ContextEntry, ContextStore};
use crate::embeddings::EmbeddingService;
use crate::providers::LLMProvider;

/// Chat agent that provides interactive conversation with an LLM
#[derive(Debug)]
pub struct ChatAgent {
    provider: LLMProvider,
    config: AgentConfig,
    embedding_service: EmbeddingService,
}

impl ChatAgent {
    /// Create a new chat agent with the given configuration
    pub fn new(config: AgentConfig) -> Result<Self> {
        if config.verbose {
            info!(
                "Initializing {} client with model: {}",
                config.provider, config.model
            );
        }

        let provider =
            LLMProvider::new(&config.provider, &config.model, config.api_key.as_deref())?;

        let embedding_service = EmbeddingService::new(384); // Standard embedding dimension

        Ok(ChatAgent {
            provider,
            config,
            embedding_service,
        })
    }

    /// Get a reference to the agent's configuration
    pub fn config(&self) -> &AgentConfig {
        &self.config
    }

    /// Get a response from the AI for the given prompt with context
    async fn get_response(
        &self,
        prompt: &str,
        context: &ContextStore,
        session_id: &str,
    ) -> Result<String> {
        if self.config.verbose {
            debug!("Sending prompt to AI model with context");
        }

        // Generate embedding for the current prompt
        let query_embedding = self.embedding_service.embed(prompt).await?;

        // Retrieve relevant context from previous conversations
        let relevant_context = context
            .get_relevant_context(query_embedding, Some(session_id), 5)
            .await?;

        // Build context-aware preamble
        let mut preamble =
            "You are a helpful AI assistant. Respond in a conversational and helpful manner."
                .to_string();

        if !relevant_context.is_empty() {
            preamble.push_str("\n\nHere is some relevant context from our previous conversations:");
            for entry in &relevant_context {
                preamble.push_str(&format!(
                    "\n[{}] {}: {}",
                    entry.timestamp.format("%H:%M"),
                    entry.role,
                    entry.content
                ));
            }
            preamble.push_str("\n\nPlease use this context to provide a more informed response.");
        }

        let response = self.provider.prompt(prompt, &preamble, 2048).await?;

        if self.config.verbose {
            debug!("Received response from AI model");
        }

        Ok(response)
    }

    /// Handle slash commands
    async fn handle_command(
        &self,
        command: &str,
        context: &ContextStore,
        current_session_id: &str,
    ) -> Result<Option<String>> {
        let parts: Vec<&str> = command.trim_start_matches('/').split_whitespace().collect();
        if parts.is_empty() {
            return Ok(None);
        }

        match parts[0] {
            "quit" => {
                println!("Goodbye!");
                std::process::exit(0);
            }
            "new" => {
                let new_session_id = Uuid::new_v4().to_string();
                println!("Starting new session with ID: {}", new_session_id);
                println!(
                    "(Use /session {} to return to this session)",
                    new_session_id
                );
                return Ok(Some(new_session_id));
            }
            "session" => {
                if parts.len() == 1 {
                    println!("Current session ID: {}", current_session_id);
                } else if parts.len() == 2 {
                    let target_session = parts[1];
                    if context.session_exists(target_session).await? {
                        println!("Switching to session: {}", target_session);
                        return Ok(Some(target_session.to_string()));
                    } else {
                        println!(
                            "Session '{}' not found. Use /sessions to list available sessions.",
                            target_session
                        );
                    }
                } else {
                    println!("Usage: /session [session_id]");
                }
            }
            "sessions" => {
                let sessions = context.list_sessions().await?;
                if sessions.is_empty() {
                    println!("No sessions found.");
                } else {
                    println!("Available sessions:");
                    for session in sessions {
                        let current_marker = if session.session_id == current_session_id {
                            " (current)"
                        } else {
                            ""
                        };
                        println!(
                            "  {} - {} entries, last active: {}{}",
                            session.session_id,
                            session.entry_count,
                            session.last_entry.format("%Y-%m-%d %H:%M:%S UTC"),
                            current_marker
                        );
                    }
                }
            }
            _ => {
                println!("Unknown command: /{}", parts[0]);
                println!("Available commands: /quit, /new, /session, /sessions, /session <id>");
            }
        }

        Ok(None)
    }
}

#[async_trait]
impl Agent for ChatAgent {
    async fn run(&self, context: &ContextStore, session_id: &str) -> Result<Option<String>> {
        if self.config.verbose {
            info!("Starting chat session with ID: {}", session_id);
        }

        println!("Chat with AI Agent (use 'quit', '/quit' or Ctrl+C to exit)");
        println!("Available commands: /quit, /new, /session, /sessions, /session <id>");
        println!();

        // Show the agent's greeting
        println!("\x1b[93mAgent\x1b[0m: {}", self.greeting());
        println!();

        loop {
            // Get user input
            print!("\x1b[94mλ\x1b[0m ");
            io::stdout().flush()?;

            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(0) => {
                    if self.config.verbose {
                        debug!("EOF received, ending chat session");
                    }
                    break;
                }
                Ok(_) => {
                    let user_input = input.trim();

                    // Check for quit commands
                    if user_input.is_empty() {
                        if self.config.verbose {
                            debug!("Skipping empty message");
                        }
                        continue;
                    }

                    if user_input.eq_ignore_ascii_case("quit")
                        || user_input.eq_ignore_ascii_case("exit")
                    {
                        if self.config.verbose {
                            info!("User requested to quit");
                        }
                        break;
                    }

                    // Handle slash commands
                    if user_input.starts_with('/') {
                        match self.handle_command(user_input, context, session_id).await {
                            Ok(Some(new_session_id)) => {
                                if self.config.verbose {
                                    info!("Switching to session: {}", new_session_id);
                                }
                                return Ok(Some(new_session_id));
                            }
                            Ok(None) => {
                                // Command handled, continue loop
                                continue;
                            }
                            Err(e) => {
                                error!("Error handling command: {}", e);
                                println!("\x1b[91mError\x1b[0m: Failed to handle command");
                            }
                        }
                        continue;
                    }

                    if self.config.verbose {
                        debug!("User input received: {:?}", user_input);
                    }

                    // Store user input in context
                    let user_entry = ContextEntry::new(
                        self.name().to_string(),
                        session_id.to_string(),
                        user_input.to_string(),
                        "user".to_string(),
                    );

                    let user_embedding = self.embedding_service.embed(user_input).await?;
                    if let Err(e) = context.store_context(user_entry, user_embedding).await {
                        warn!("Failed to store user context: {}", e);
                    }

                    // Send message to AI and get response
                    match self.get_response(user_input, context, session_id).await {
                        Ok(response) => {
                            println!("\x1b[93mAgent\x1b[0m: {}", response);
                            println!();

                            // Store agent response in context
                            let agent_entry = ContextEntry::new(
                                self.name().to_string(),
                                session_id.to_string(),
                                response.clone(),
                                "assistant".to_string(),
                            );

                            let agent_embedding = self.embedding_service.embed(&response).await?;
                            if let Err(e) =
                                context.store_context(agent_entry, agent_embedding).await
                            {
                                warn!("Failed to store agent context: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Error getting response: {}", e);
                            println!("\x1b[91mError\x1b[0m: Failed to get response from AI agent");
                            println!();
                        }
                    }
                }
                Err(e) => {
                    error!("Error reading input: {}", e);
                    return Err(e.into());
                }
            }
        }

        if self.config.verbose {
            info!("Chat session ended");
        }

        Ok(None)
    }

    fn name(&self) -> &'static str {
        "chat"
    }

    fn greeting(&self) -> &'static str {
        "What can I help you with?"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config(provider: &str, model: &str, api_key: Option<String>) -> AgentConfig {
        AgentConfig::new(false, provider.to_string(), model.to_string(), api_key)
    }

    #[test]
    fn test_chat_agent_creation_with_ollama() {
        let config = create_test_config("ollama", "llama3.2", None);
        let agent = ChatAgent::new(config);

        assert!(agent.is_ok());
        let agent = agent.unwrap();
        assert_eq!(agent.name(), "chat");
    }

    #[test]
    fn test_chat_agent_creation_with_openrouter() {
        let config = create_test_config("openrouter", "gpt-4", Some("test-api-key".to_string()));
        let agent = ChatAgent::new(config);

        assert!(agent.is_ok());
        let agent = agent.unwrap();
        assert_eq!(agent.name(), "chat");
    }

    #[test]
    fn test_chat_agent_creation_with_invalid_provider() {
        let config = create_test_config("invalid", "model", None);
        let agent = ChatAgent::new(config);

        assert!(agent.is_err());
        let error = agent.unwrap_err();
        assert!(error.to_string().contains("Unsupported provider: invalid"));
    }

    #[test]
    fn test_chat_agent_creation_openrouter_without_api_key() {
        let config = create_test_config("openrouter", "gpt-4", None);
        let agent = ChatAgent::new(config);

        assert!(agent.is_err());
        let error = agent.unwrap_err();
        assert!(error.to_string().contains("OpenRouter API key is required"));
    }

    #[test]
    fn test_chat_agent_verbose_config() {
        let mut config = create_test_config("ollama", "llama3.2", None);
        config.verbose = true;

        let agent = ChatAgent::new(config);
        assert!(agent.is_ok());

        let agent = agent.unwrap();
        assert_eq!(agent.config().verbose, true);
    }

    #[test]
    fn test_chat_agent_config_preservation() {
        let config = create_test_config("ollama", "test-model", None);
        let original_provider = config.provider.clone();
        let original_model = config.model.clone();

        let agent = ChatAgent::new(config).unwrap();

        assert_eq!(agent.config().provider, original_provider);
        assert_eq!(agent.config().model, original_model);
        assert_eq!(agent.provider.model(), "test-model");
    }
}
