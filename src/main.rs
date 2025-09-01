use anyhow::Result;
use clap::Parser;
use std::env;
use std::path::PathBuf;
use tracing::{debug, error, info};
use uuid::Uuid;

pub mod agents;
pub mod context;
pub mod embeddings;
pub mod providers;
pub mod tools;
pub mod web;

use crate::web::start_web_server;
use agents::chat::ChatAgent;
use agents::{Agent, AgentConfig};
use context::ContextStore;

#[derive(Parser, Debug)]
#[command(
    name = "ally",
    about = "An AI chat agent built with Rust and the Rig framework",
    long_about = "Ally is a command-line AI chat agent that supports multiple LLM providers including Ollama and OpenRouter. \
                  It provides an interactive chat interface with persistent context across sessions.\n\n\
                  Environment Variables:\n\
                  - ALLY_PROVIDER: Set the LLM provider (ollama, openrouter)\n\
                  - ALLY_MODEL: Set the model name\n\
                  - ALLY_EMBEDDING_PROVIDER: Set the embedding provider (openai, ollama, simple)\n\
                  - ALLY_EMBEDDING_MODEL: Set the embedding model name\n\
                  - ALLY_CONTEXT_DB: Set the context database path\n\
                  - ALLY_SESSION_ID: Set the session ID for context sharing\n\
                  - OPENROUTER_API_KEY: Set the OpenRouter API key\n\
                  - OPENAI_API_KEY: Set the OpenAI API key for embeddings"
)]
struct Args {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// LLM provider to use (ollama or openrouter)
    /// Can also be set via ALLY_PROVIDER environment variable
    #[arg(short, long, env = "ALLY_PROVIDER", default_value = "ollama")]
    provider: String,

    /// Model name to use
    /// Can also be set via ALLY_MODEL environment variable
    #[arg(short, long, env = "ALLY_MODEL", default_value = "llama3.2")]
    model: String,

    /// OpenRouter API key (required if using openrouter provider)
    /// Can also be set via OPENROUTER_API_KEY environment variable
    #[arg(long, env)]
    openrouter_api_key: Option<String>,

    /// Embedding provider to use (openai, ollama, or simple)
    /// Can also be set via ALLY_EMBEDDING_PROVIDER environment variable
    #[arg(long, env = "ALLY_EMBEDDING_PROVIDER", default_value = "simple")]
    embedding_provider: String,

    /// Embedding model name to use
    /// Can also be set via ALLY_EMBEDDING_MODEL environment variable
    #[arg(long, env = "ALLY_EMBEDDING_MODEL")]
    embedding_model: Option<String>,

    /// OpenAI API key (required if using openai embedding provider)
    /// Can also be set via OPENAI_API_KEY environment variable
    #[arg(long, env)]
    openai_api_key: Option<String>,

    /// Path to the context database file
    /// Can also be set via ALLY_CONTEXT_DB environment variable
    #[arg(long, env = "ALLY_CONTEXT_DB", default_value = "./ally_context.db")]
    context_db: PathBuf,

    /// Session ID for context sharing (generates new if not provided)
    /// Can also be set via ALLY_SESSION_ID environment variable
    #[arg(long, env = "ALLY_SESSION_ID")]
    session_id: Option<String>,

    /// Port for the web server (default: 3000)
    #[arg(long, default_value = "3000")]
    web_port: u16,

    /// Skip tool execution confirmation prompts (YOLO mode)
    #[arg(long)]
    yolo: bool,
}

impl Args {
    /// Display configuration values from command line arguments and environment variables
    fn display_configuration(&self) {
        println!("🚀 Ally Configuration");
        println!("═══════════════════════════════════════════════════════════════");

        // Display command line arguments
        println!("📋 Command Line Arguments:");
        println!(
            "  • Verbose logging: {}",
            if self.verbose { "enabled" } else { "disabled" }
        );
        println!("  • LLM provider: {}", self.provider);
        println!("  • LLM model: {}", self.model);
        println!("  • Embedding provider: {}", self.embedding_provider);
        if let Some(ref model) = self.embedding_model {
            println!("  • Embedding model: {}", model);
        } else {
            println!("  • Embedding model: <default>");
        }
        println!("  • Context database: {}", self.context_db.display());
        if let Some(ref session) = self.session_id {
            println!("  • Session ID: {}", session);
        } else {
            println!("  • Session ID: <will be generated>");
        }
        println!("  • Web server port: {}", self.web_port);
        println!(
            "  • YOLO mode: {}",
            if self.yolo { "enabled" } else { "disabled" }
        );

        // Display API key status (without revealing the actual keys)
        if self.openrouter_api_key.is_some() {
            println!("  • OpenRouter API key: ✓ configured");
        } else {
            println!("  • OpenRouter API key: ✗ not set");
        }

        if self.openai_api_key.is_some() {
            println!("  • OpenAI API key: ✓ configured");
        } else {
            println!("  • OpenAI API key: ✗ not set");
        }

        println!();

        // Display environment variables
        println!("🌍 Environment Variables:");
        let env_vars = [
            ("ALLY_PROVIDER", "LLM provider"),
            ("ALLY_MODEL", "LLM model"),
            ("ALLY_EMBEDDING_PROVIDER", "Embedding provider"),
            ("ALLY_EMBEDDING_MODEL", "Embedding model"),
            ("ALLY_CONTEXT_DB", "Context database path"),
            ("ALLY_SESSION_ID", "Session ID"),
            ("OPENROUTER_API_KEY", "OpenRouter API key"),
            ("OPENAI_API_KEY", "OpenAI API key"),
        ];

        for (var_name, description) in &env_vars {
            match env::var(var_name) {
                Ok(value) => {
                    if var_name.contains("API_KEY") {
                        println!("  • {} ({}): ✓ configured", var_name, description);
                    } else {
                        println!("  • {} ({}): {}", var_name, description, value);
                    }
                }
                Err(_) => {
                    println!("  • {} ({}): ✗ not set", var_name, description);
                }
            }
        }

        println!("═══════════════════════════════════════════════════════════════");
        println!();
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Display configuration at startup
    args.display_configuration();

    // Initialize logging
    let filter = if args.verbose { "debug" } else { "info" };

    tracing_subscriber::fmt().with_env_filter(filter).init();

    if args.verbose {
        info!("Verbose logging enabled");
        debug!("Arguments: {:?}", args);
    }

    // Generate or use provided session ID
    let is_new_session = args.session_id.is_none();
    let session_id = args
        .session_id
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    if args.verbose {
        info!("Context database: {:?}", args.context_db);
        if is_new_session {
            info!("Generated new session ID: {}", session_id);
        } else {
            info!("Using existing session ID: {}", session_id);
        }
    }

    // Create embedding provider to determine dimension
    let embedding_provider = crate::embeddings::EmbeddingProvider::new(
        &args.embedding_provider,
        args.embedding_model.as_deref(),
        args.openai_api_key.as_deref(),
    )?;
    let embedding_dimension = embedding_provider.create_service().dimension();

    // Initialize context store with correct embedding dimension
    let context = ContextStore::new(&args.context_db, embedding_dimension).await?;
    let context_arc = std::sync::Arc::new(context);

    // Create agent configuration
    let config = AgentConfig::new(
        args.verbose,
        args.provider,
        args.model,
        args.openrouter_api_key,
        args.embedding_provider,
        args.embedding_model,
        args.openai_api_key,
        args.yolo,
    );

    // Start web server in background
    let web_context = context_arc.clone();
    let web_port = args.web_port;
    tokio::spawn(async move {
        if let Err(e) = start_web_server(web_context, web_port).await {
            error!("Web server error: {}", e);
        }
    });

    info!(
        "Web interface available at http://127.0.0.1:{}",
        args.web_port
    );

    // Create the chat agent
    let agent = ChatAgent::new(config)?;

    // Main session loop to handle session switching
    let mut current_session_id = session_id;
    let mut is_new_session_flag = is_new_session;

    loop {
        // Print session information to user
        if is_new_session_flag {
            println!("Starting new session with ID: {}", current_session_id);
            println!(
                "(Use --session-id {} to resume this session later)",
                current_session_id
            );
        } else {
            println!("Resuming session: {}", current_session_id);
        }
        println!();

        // Run the agent
        match agent.run(&*context_arc, &current_session_id).await? {
            Some(new_session_id) => {
                // Agent requested session switch
                current_session_id = new_session_id;
                is_new_session_flag = true; // Treat switched sessions as new for display purposes
                println!(); // Add spacing between sessions
            }
            None => {
                // Agent exited normally
                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_default_args() {
        // Temporarily unset environment variables for this test
        unsafe {
            std::env::remove_var("ALLY_PROVIDER");
            std::env::remove_var("ALLY_MODEL");
            std::env::remove_var("OPENROUTER_API_KEY");
        }

        let args = Args::try_parse_from(&["ally"]).unwrap();

        assert_eq!(args.verbose, false);
        assert_eq!(args.provider, "ollama");
        assert_eq!(args.model, "llama3.2");
        assert_eq!(args.openrouter_api_key, None);
        assert_eq!(args.yolo, false);
    }

    #[test]
    fn test_verbose_flag() {
        let args = Args::try_parse_from(&["ally", "--verbose"]).unwrap();
        assert_eq!(args.verbose, true);

        let args = Args::try_parse_from(&["ally", "-v"]).unwrap();
        assert_eq!(args.verbose, true);
    }

    #[test]
    fn test_provider_option() {
        let args = Args::try_parse_from(&["ally", "--provider", "openrouter"]).unwrap();
        assert_eq!(args.provider, "openrouter");

        let args = Args::try_parse_from(&["ally", "-p", "ollama"]).unwrap();
        assert_eq!(args.provider, "ollama");
    }

    #[test]
    fn test_model_option() {
        let args = Args::try_parse_from(&["ally", "--model", "gpt-4"]).unwrap();
        assert_eq!(args.model, "gpt-4");

        let args = Args::try_parse_from(&["ally", "-m", "llama3.1"]).unwrap();
        assert_eq!(args.model, "llama3.1");
    }

    #[test]
    fn test_openrouter_api_key() {
        let args = Args::try_parse_from(&["ally", "--openrouter-api-key", "test-key"]).unwrap();
        assert_eq!(args.openrouter_api_key, Some("test-key".to_string()));
    }

    #[test]
    fn test_combined_args() {
        let args = Args::try_parse_from(&[
            "ally",
            "--verbose",
            "--provider",
            "openrouter",
            "--model",
            "gpt-4",
            "--openrouter-api-key",
            "test-key",
        ])
        .unwrap();

        assert_eq!(args.verbose, true);
        assert_eq!(args.provider, "openrouter");
        assert_eq!(args.model, "gpt-4");
        assert_eq!(args.openrouter_api_key, Some("test-key".to_string()));
    }

    #[test]
    fn test_agent_config_from_args() {
        let args = Args {
            verbose: true,
            provider: "ollama".to_string(),
            model: "llama3.2".to_string(),
            openrouter_api_key: None,
            embedding_provider: "simple".to_string(),
            embedding_model: None,
            openai_api_key: None,
            context_db: "./test_context.db".into(),
            session_id: Some("test_session".to_string()),
            web_port: 3000,
            yolo: false,
        };

        let config = AgentConfig::new(
            args.verbose,
            args.provider,
            args.model,
            args.openrouter_api_key,
            args.embedding_provider,
            args.embedding_model,
            args.openai_api_key,
            args.yolo,
        );

        assert_eq!(config.verbose, true);
        assert_eq!(config.provider, "ollama");
        assert_eq!(config.model, "llama3.2");
        assert_eq!(config.api_key, None);
        assert_eq!(config.embedding_provider, "simple");
        assert_eq!(config.embedding_model, None);
        assert_eq!(config.openai_api_key, None);
    }
}
