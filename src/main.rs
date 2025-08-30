use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::{debug, info};
use uuid::Uuid;

pub mod agents;
pub mod context;
pub mod embeddings;
pub mod providers;

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
                  - ALLY_CONTEXT_DB: Set the context database path\n\
                  - ALLY_SESSION_ID: Set the session ID for context sharing\n\
                  - OPENROUTER_API_KEY: Set the OpenRouter API key"
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

    /// Path to the context database file
    /// Can also be set via ALLY_CONTEXT_DB environment variable
    #[arg(long, env = "ALLY_CONTEXT_DB", default_value = "./ally_context.db")]
    context_db: PathBuf,

    /// Session ID for context sharing (generates new if not provided)
    /// Can also be set via ALLY_SESSION_ID environment variable
    #[arg(long, env = "ALLY_SESSION_ID")]
    session_id: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

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

    // Initialize context store
    let context = ContextStore::new(&args.context_db, 384).await?;

    // Create agent configuration
    let config = AgentConfig::new(
        args.verbose,
        args.provider,
        args.model,
        args.openrouter_api_key,
    );

    // Create and run the chat agent
    let agent = ChatAgent::new(config)?;

    // Print session information to user
    if is_new_session {
        println!("Starting new session with ID: {}", session_id);
        println!(
            "(Use --session-id {} to resume this session later)",
            session_id
        );
    } else {
        println!("Resuming session: {}", session_id);
    }
    println!();

    agent.run(&context, &session_id).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_default_args() {
        let args = Args::try_parse_from(&["ally"]).unwrap();

        assert_eq!(args.verbose, false);
        assert_eq!(args.provider, "ollama");
        assert_eq!(args.model, "llama3.2");
        assert_eq!(args.openrouter_api_key, None);
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
            context_db: "./test_context.db".into(),
            session_id: Some("test_session".to_string()),
        };

        let config = AgentConfig::new(
            args.verbose,
            args.provider,
            args.model,
            args.openrouter_api_key,
        );

        assert_eq!(config.verbose, true);
        assert_eq!(config.provider, "ollama");
        assert_eq!(config.model, "llama3.2");
        assert_eq!(config.api_key, None);
    }
}
