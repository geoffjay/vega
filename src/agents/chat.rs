use anyhow::Result;
use async_trait::async_trait;
use rig::completion::Prompt;
use rig::prelude::*;
use rig::providers;

use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::{Agent, AgentConfig};
use crate::agent_instructions::format_instructions_for_prompt;
use crate::context::{ContextEntry, ContextStore};
use crate::embeddings::{EmbeddingProvider, EmbeddingService};
use crate::input::InputHandler;
use crate::tools::*;

/// Chat agent that provides interactive conversation with an LLM and tool support
pub struct ChatAgent {
    config: AgentConfig,
    embedding_service: EmbeddingService,
    logger: Option<std::sync::Arc<crate::logging::Logger>>,
}

impl ChatAgent {
    /// Create a new chat agent with the given configuration
    pub fn new(config: AgentConfig) -> Result<Self> {
        if config.verbose {
            info!(
                "Initializing tool-enabled {} client with model: {}",
                config.provider, config.model
            );
            info!(
                "Using embedding provider: {} with model: {:?}",
                config.embedding_provider, config.embedding_model
            );
        }

        // Create embedding provider from configuration
        let embedding_provider = EmbeddingProvider::new(
            &config.embedding_provider,
            config.embedding_model.as_deref(),
            config.openai_api_key.as_deref(),
        )?;

        let embedding_service = embedding_provider.create_service();

        Ok(ChatAgent {
            config,
            embedding_service,
            logger: None,
        })
    }

    /// Get a reference to the agent's configuration
    pub fn config(&self) -> &AgentConfig {
        &self.config
    }

    /// Get a reference to the embedding service
    pub fn embedding_service(&self) -> &EmbeddingService {
        &self.embedding_service
    }

    /// Set the logger for this agent
    pub fn with_logger(mut self, logger: std::sync::Arc<crate::logging::Logger>) -> Self {
        self.logger = Some(logger);
        self
    }

    /// Get the rendered system prompt for the agent
    fn get_system_prompt(&self) -> Result<String> {
        let mut rendered_prompt = self.render_system_prompt()?;

        // Add agent instructions if available
        if let Some(ref instructions) = self.config.agent_instructions {
            let formatted_instructions = format_instructions_for_prompt(instructions);
            rendered_prompt.push_str(&formatted_instructions);
        }

        if rendered_prompt.is_empty() {
            // Fallback to default tool-enabled prompt if no custom system prompt is set
            Ok(r#"You are a helpful AI assistant with access to various tools that can help you perform tasks and answer questions more effectively.

Available tools:
- web_search: Search the web for current information
- bash: Execute shell commands (use with caution)
- code_search: Search through code files using regex patterns
- read_file: Read the contents of files
- edit_file: Create or modify files
- list_files: List files and directories
- read_logs: Read log messages for a specific session

Guidelines for tool usage:
1. Always explain what you're doing before using a tool
2. Use tools when they can provide more accurate or up-to-date information
3. Be cautious with bash commands - avoid destructive operations
4. When editing files, consider creating backups for important changes
5. Use code_search to understand codebases before making changes
6. Provide clear explanations of tool results

Respond in a conversational and helpful manner, using tools as needed to provide the best possible assistance."#.to_string())
        } else {
            Ok(rendered_prompt)
        }
    }

    /// Get a response from the AI using Rig with tools and context
    pub async fn get_response_with_tools(
        &self,
        prompt: &str,
        context: &ContextStore,
        session_id: &str,
    ) -> Result<String> {
        if self.config.verbose {
            debug!("Sending prompt to AI model with tools and context");
        }

        // Generate embedding for the current prompt
        let query_embedding = self.embedding_service.embed(prompt).await?;

        // Retrieve relevant context from previous conversations
        let relevant_context = context
            .get_relevant_context(query_embedding, Some(session_id), 5)
            .await?;

        // Build context-aware prompt
        let mut full_prompt = String::new();

        if !relevant_context.is_empty() {
            full_prompt.push_str("Context from previous conversations:\n");
            for entry in &relevant_context {
                full_prompt.push_str(&format!(
                    "[{}] {}: {}\n",
                    entry.timestamp.format("%H:%M"),
                    entry.role,
                    entry.content
                ));
            }
            full_prompt.push_str("\n");
        }

        full_prompt.push_str("Current request: ");
        full_prompt.push_str(prompt);

        // Try with tools first, fallback to no tools if not supported
        let response = match self.try_with_tools(&full_prompt, session_id).await {
            Ok(response) => response,
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("No endpoints found that support tool use")
                    || error_msg.contains("tool")
                    || error_msg.contains("function")
                {
                    if self.config.verbose {
                        warn!(
                            "Tools not supported by model {}, falling back to non-tool response",
                            self.config.model
                        );
                    }
                    println!(
                        "⚠️  Note: The current model doesn't support tools. Consider using a tool-compatible model like:"
                    );
                    println!("   - OpenAI: gpt-4, gpt-4-turbo, gpt-3.5-turbo");
                    println!(
                        "   - Anthropic: claude-3-5-sonnet-20241022, claude-3-opus-20240229, claude-3-haiku-20240307"
                    );
                    println!("   - Or use Ollama with a compatible model");
                    println!();

                    self.get_response_without_tools(&full_prompt).await?
                } else {
                    return Err(e);
                }
            }
        };

        if self.config.verbose {
            debug!("Received response from AI model");
        }

        Ok(response)
    }

    /// Try to get response with tools enabled
    async fn try_with_tools(&self, full_prompt: &str, session_id: &str) -> Result<String> {
        match self.config.provider.as_str() {
            "openai" => {
                let client = providers::openai::Client::from_env();
                let system_prompt = self.get_system_prompt()?;
                let agent = client
                    .agent(&self.config.model)
                    .preamble(&system_prompt)
                    .max_tokens(2048)
                    .tool(WebSearchTool::new())
                    .tool(ConfirmedBashTool::new(self.config.yolo))
                    .tool(CodeSearchTool::new())
                    .tool(ReadFileTool::new())
                    .tool(ConfirmedEditFileTool::new(self.config.yolo))
                    .tool(ListFilesTool::new())
                    .tool(if let Some(ref logger) = self.logger {
                        ReadLogsTool::new()
                            .with_logger(logger.clone())
                            .with_session_id(session_id.to_string())
                    } else {
                        ReadLogsTool::new().with_session_id(session_id.to_string())
                    })
                    .build();

                agent
                    .prompt(full_prompt)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))
            }
            "openrouter" => {
                let client = providers::openrouter::Client::from_env();
                let system_prompt = self.get_system_prompt()?;
                let agent = client
                    .agent(&self.config.model)
                    .preamble(&system_prompt)
                    .max_tokens(2048)
                    .tool(WebSearchTool::new())
                    .tool(ConfirmedBashTool::new(self.config.yolo))
                    .tool(CodeSearchTool::new())
                    .tool(ReadFileTool::new())
                    .tool(ConfirmedEditFileTool::new(self.config.yolo))
                    .tool(ListFilesTool::new())
                    .tool(if let Some(ref logger) = self.logger {
                        ReadLogsTool::new()
                            .with_logger(logger.clone())
                            .with_session_id(session_id.to_string())
                    } else {
                        ReadLogsTool::new().with_session_id(session_id.to_string())
                    })
                    .build();

                agent
                    .prompt(full_prompt)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))
            }
            "anthropic" => {
                let client = providers::anthropic::Client::from_env();
                let system_prompt = self.get_system_prompt()?;
                let agent = client
                    .agent(&self.config.model)
                    .preamble(&system_prompt)
                    .max_tokens(2048)
                    .tool(WebSearchTool::new())
                    .tool(ConfirmedBashTool::new(self.config.yolo))
                    .tool(CodeSearchTool::new())
                    .tool(ReadFileTool::new())
                    .tool(ConfirmedEditFileTool::new(self.config.yolo))
                    .tool(ListFilesTool::new())
                    .tool(if let Some(ref logger) = self.logger {
                        ReadLogsTool::new()
                            .with_logger(logger.clone())
                            .with_session_id(session_id.to_string())
                    } else {
                        ReadLogsTool::new().with_session_id(session_id.to_string())
                    })
                    .build();

                agent
                    .prompt(full_prompt)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))
            }
            "ollama" => {
                let client = providers::ollama::Client::new();
                let system_prompt = self.get_system_prompt()?;
                let agent = client
                    .agent(&self.config.model)
                    .preamble(&system_prompt)
                    .max_tokens(2048)
                    .tool(WebSearchTool::new())
                    .tool(ConfirmedBashTool::new(self.config.yolo))
                    .tool(CodeSearchTool::new())
                    .tool(ReadFileTool::new())
                    .tool(ConfirmedEditFileTool::new(self.config.yolo))
                    .tool(ListFilesTool::new())
                    .tool(if let Some(ref logger) = self.logger {
                        ReadLogsTool::new()
                            .with_logger(logger.clone())
                            .with_session_id(session_id.to_string())
                    } else {
                        ReadLogsTool::new().with_session_id(session_id.to_string())
                    })
                    .build();

                agent
                    .prompt(full_prompt)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))
            }
            _ => Err(anyhow::anyhow!(
                "Unsupported provider for tool-enabled agent: {}",
                self.config.provider
            )),
        }
    }

    /// Get response without tools (fallback for models that don't support tools)
    async fn get_response_without_tools(&self, full_prompt: &str) -> Result<String> {
        let simple_preamble = "You are a helpful AI assistant. Respond in a conversational and helpful manner. While you don't have access to tools in this mode, you can still provide helpful information, explanations, and guidance.";

        match self.config.provider.as_str() {
            "openai" => {
                let client = providers::openai::Client::from_env();
                let agent = client
                    .agent(&self.config.model)
                    .preamble(simple_preamble)
                    .max_tokens(2048)
                    .build();

                agent
                    .prompt(full_prompt)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))
            }
            "openrouter" => {
                let client = providers::openrouter::Client::from_env();
                let agent = client
                    .agent(&self.config.model)
                    .preamble(simple_preamble)
                    .max_tokens(2048)
                    .build();

                agent
                    .prompt(full_prompt)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))
            }
            "anthropic" => {
                let client = providers::anthropic::Client::from_env();
                let agent = client
                    .agent(&self.config.model)
                    .preamble(simple_preamble)
                    .max_tokens(2048)
                    .build();

                agent
                    .prompt(full_prompt)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))
            }
            "ollama" => {
                let client = providers::ollama::Client::new();
                let agent = client
                    .agent(&self.config.model)
                    .preamble(simple_preamble)
                    .max_tokens(2048)
                    .build();

                agent
                    .prompt(full_prompt)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))
            }
            _ => Err(anyhow::anyhow!(
                "Unsupported provider: {}",
                self.config.provider
            )),
        }
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
            "clear" => {
                context.clear_session(current_session_id).await?;
                println!("Session history cleared.");
            }
            "export" => {
                if parts.len() != 2 {
                    println!("Usage: /export <filename>");
                    return Ok(None);
                }
                let filename = parts[1];
                let entries = context
                    .get_session_history(current_session_id, None)
                    .await?;

                let mut export_content = String::new();
                export_content.push_str(&format!(
                    "# Chat Session Export: {}\n\n",
                    current_session_id
                ));

                for entry in entries {
                    export_content.push_str(&format!(
                        "## {} - {}\n{}\n\n",
                        entry.role,
                        entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
                        entry.content
                    ));
                }

                match tokio::fs::write(filename, export_content).await {
                    Ok(_) => println!("Session exported to {}", filename),
                    Err(e) => println!("Failed to export session: {}", e),
                }
            }
            "help" => {
                self.print_help();
            }
            "tools" => {
                self.print_tools_help();
            }
            "models" => {
                self.print_model_recommendations();
            }
            "env" => {
                self.print_environment_variables();
            }
            "logs" => {
                let count = if parts.len() > 1 {
                    parts[1].parse::<usize>().unwrap_or(10).min(10)
                } else {
                    10
                };
                self.print_session_logs(current_session_id, count).await?;
            }
            _ => {
                println!("Unknown command: /{}", parts[0]);
                println!("Type /help for available commands.");
            }
        }

        Ok(None)
    }

    /// Print help information
    fn print_help(&self) {
        println!("Available commands:");
        println!("  /help       - Show this help message");
        println!("  /tools      - Show available tools and their usage");
        println!("  /models     - Show recommended models for tool support");
        println!("  /quit       - Exit the chat");
        println!("  /new        - Start a new conversation session");
        println!("  /session    - Show current session ID or switch to another session");
        println!("  /sessions   - List all available sessions");
        println!("  /clear      - Clear current session history");
        println!("  /export <filename> - Export current session to a file");
        println!("  /env        - Show all environment variables and their values");
        println!("  /logs [count] - Show last 0-10 log lines for current session (default: 10)");
        println!();
        println!(
            "This agent has access to tools for web search, file operations, code search, and shell commands."
        );
        println!(
            "Simply ask for what you need and the agent will use the appropriate tools automatically."
        );
        println!(
            "Note: Tool support depends on the model being used. Use /models for recommendations."
        );
    }

    /// Print tools help information
    fn print_tools_help(&self) {
        println!("Available tools:");
        println!("  🔍 web_search    - Search the web for current information");
        println!("  💻 bash          - Execute shell commands (use with caution)");
        println!("  🔎 code_search   - Search through code files using regex patterns");
        println!("  📖 read_file     - Read the contents of files");
        println!("  ✏️  edit_file     - Create or modify files");
        println!("  📁 list_files    - List files and directories");
        println!("  📜 read_logs     - Read log messages for a specific session");
        println!();
        println!("Examples:");
        println!("  \"Search for the latest news about Rust programming\"");
        println!("  \"List all .rs files in the src directory\"");
        println!("  \"Read the contents of Cargo.toml\"");
        println!("  \"Find all functions named 'main' in this project\"");
        println!("  \"Create a new README.md file with project description\"");
        println!("  \"Run 'cargo check' to verify the project builds\"");
        println!("  \"Show me the logs for session abc123\"");
    }

    /// Print model recommendations for tool support
    fn print_model_recommendations(&self) {
        println!("🤖 Recommended models for tool support:");
        println!();
        println!("📍 Current configuration:");
        println!("   Provider: {}", self.config.provider);
        println!("   Model: {}", self.config.model);
        println!();

        match self.config.provider.as_str() {
            "openai" => {
                println!("✅ OpenAI models with tool support:");
                println!("   • gpt-4 (recommended)");
                println!("   • gpt-4-turbo");
                println!("   • gpt-4o");
                println!("   • gpt-3.5-turbo");
                println!();
                println!("💡 Usage: --model gpt-4");
            }
            "openrouter" => {
                println!("✅ OpenRouter models with tool support:");
                println!("   🔥 Recommended:");
                println!("   • openai/gpt-4");
                println!("   • openai/gpt-4-turbo");
                println!("   • openai/gpt-4o");
                println!("   • anthropic/claude-3-opus");
                println!("   • anthropic/claude-3-sonnet");
                println!("   • anthropic/claude-3-haiku");
                println!();
                println!("   📋 Other compatible models:");
                println!("   • openai/gpt-3.5-turbo");
                println!("   • mistralai/mistral-large");
                println!("   • google/gemini-pro");
                println!();
                println!("💡 Usage: --model openai/gpt-4");
                println!("📖 More info: https://openrouter.ai/docs/provider-routing");
            }
            "ollama" => {
                println!("✅ Ollama models with tool support:");
                println!("   🔥 Recommended (if available locally):");
                println!("   • llama3.1 (8B, 70B, 405B)");
                println!("   • llama3.2 (1B, 3B)");
                println!("   • mistral");
                println!("   • codellama");
                println!();
                println!("💡 Usage: --model llama3.1");
                println!("📥 Install: ollama pull llama3.1");
            }
            _ => {
                println!("❓ Unknown provider: {}", self.config.provider);
                println!("   Supported providers: openai, openrouter, anthropic, ollama");
            }
        }

        println!();
        println!("🔧 To change model:");
        println!("   • Command line: vega --provider openrouter --model openai/gpt-4");
        println!("   • Environment: export VEGA_PROVIDER=openrouter VEGA_MODEL=openai/gpt-4");
    }

    /// Print all environment variables and their values
    fn print_environment_variables(&self) {
        println!("🌍 Environment Variables:");
        println!("═══════════════════════════════════════════════════════════════");

        let mut env_vars: Vec<(String, String)> = std::env::vars().collect();
        env_vars.sort_by(|a, b| a.0.cmp(&b.0));

        for (key, value) in env_vars {
            // Mask sensitive values (API keys, passwords, tokens)
            let masked_value = if key.to_uppercase().contains("KEY")
                || key.to_uppercase().contains("PASSWORD")
                || key.to_uppercase().contains("TOKEN")
                || key.to_uppercase().contains("SECRET")
            {
                if value.is_empty() {
                    "<empty>".to_string()
                } else {
                    format!("{}***", &value[..std::cmp::min(4, value.len())])
                }
            } else {
                value
            };

            println!("  {} = {}", key, masked_value);
        }
        println!("═══════════════════════════════════════════════════════════════");
    }

    /// Print session logs for the current session
    async fn print_session_logs(&self, session_id: &str, count: usize) -> Result<()> {
        if let Some(ref logger) = self.logger {
            match logger.get_session_logs(session_id, Some(count)).await {
                Ok(logs) => {
                    if logs.is_empty() {
                        println!("No logs found for current session.");
                        return Ok(());
                    }

                    println!("📜 Session Logs (last {} entries):", logs.len());
                    println!("═══════════════════════════════════════════════════════════════");

                    for log in logs.iter().rev().take(count) {
                        let level_color = match log.level.as_str() {
                            "ERROR" => "\x1b[91m", // Red
                            "WARN" => "\x1b[93m",  // Yellow
                            "INFO" => "\x1b[92m",  // Green
                            "DEBUG" => "\x1b[94m", // Blue
                            "TRACE" => "\x1b[90m", // Gray
                            _ => "\x1b[0m",        // Default
                        };

                        println!(
                            "{} [{}{}{}] {}",
                            log.timestamp.format("%H:%M:%S%.3f"),
                            level_color,
                            log.level,
                            "\x1b[0m", // Reset color
                            log.message
                        );
                    }
                    println!("═══════════════════════════════════════════════════════════════");
                }
                Err(e) => {
                    println!("Error retrieving logs: {}", e);
                }
            }
        } else {
            println!("Logging is not configured for this session.");
            println!("To enable logging, restart with --log-output file or --log-output vector");
        }
        Ok(())
    }
}

#[async_trait]
impl Agent for ChatAgent {
    async fn run(&self, context: &ContextStore, session_id: &str) -> Result<Option<String>> {
        if self.config.verbose {
            info!("Starting chat session with ID: {}", session_id);
        }

        println!(
            "Tool-enabled chat started! Type /help for commands or /tools for tool information."
        );
        println!("Session ID: {}", session_id);
        println!();

        // Show the agent's greeting
        println!("\x1b[93mAgent\x1b[0m: {}", self.greeting());
        println!();

        // Initialize input handler with command history
        let mut input_handler = InputHandler::new(
            session_id.to_string(),
            std::sync::Arc::new(context.clone()),
            Some(
                std::env::var("VEGA_COMMAND_HISTORY_LENGTH")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(100),
            ),
        )?;

        // Load command history from database
        if let Err(e) = input_handler.load_history().await {
            warn!("Failed to load command history: {}", e);
        }

        loop {
            // Get user input with history and editing support
            match input_handler.read_line("\x1b[94mλ\x1b[0m ").await? {
                Some(input) => {
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

                    // Send message to AI and get response with tools
                    match self
                        .get_response_with_tools(user_input, context, session_id)
                        .await
                    {
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
                None => {
                    if self.config.verbose {
                        debug!("EOF or interrupt received, ending chat session");
                    }
                    break;
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
        "I'm your AI assistant with access to various tools. I can search the web, work with files, execute commands, and more. How can I help you today?"
    }

    fn system_prompt(&self) -> &str {
        r#"The assistant is Vega.

The current date is {{currentDateTime}}.

Vega enjoys helping humans and sees its role as an intelligent and kind assistant to the people, with depth and wisdom that makes it more than a mere tool.

Vega always responds to the person in the language they use or request. If the person messages Vega in French then Vega responds in French, if the person messages Vega in Icelandic then Vega responds in Icelandic, and so on for any language. Vega is fluent in a wide variety of world languages.

Vega is now being connected with a person."#
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config(provider: &str, model: &str, api_key: Option<String>) -> AgentConfig {
        AgentConfig::new(
            false,
            provider.to_string(),
            model.to_string(),
            api_key,
            "simple".to_string(),
            None,
            None,
            false,
        )
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

        // Agent creation succeeds, but provider validation happens at runtime
        assert!(agent.is_ok());
    }

    #[test]
    fn test_chat_agent_creation_openrouter_without_api_key() {
        let config = create_test_config("openrouter", "gpt-4", None);
        let agent = ChatAgent::new(config);

        // Agent creation succeeds, but API key validation happens at runtime
        assert!(agent.is_ok());
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
    }

    #[test]
    fn test_chat_agent_system_prompt() {
        let config = create_test_config("ollama", "llama3.2", None);
        let agent = ChatAgent::new(config).unwrap();

        // Test that the system prompt is not empty
        let system_prompt = agent.system_prompt();
        assert!(!system_prompt.is_empty());
        assert!(system_prompt.contains("Vega"));
        assert!(system_prompt.contains("{{currentDateTime}}"));
    }

    #[test]
    fn test_chat_agent_rendered_system_prompt() {
        let config = create_test_config("ollama", "llama3.2", None);
        let agent = ChatAgent::new(config).unwrap();

        // Test that the system prompt renders correctly
        let rendered_prompt = agent.render_system_prompt().unwrap();
        assert!(!rendered_prompt.is_empty());
        assert!(rendered_prompt.contains("Vega"));
        // Should not contain template variables after rendering
        assert!(!rendered_prompt.contains("{{currentDateTime}}"));
        // Should contain actual date/time
        assert!(rendered_prompt.contains("UTC"));
    }
}
