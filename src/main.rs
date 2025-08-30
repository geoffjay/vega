use anyhow::Result;
use clap::Parser;
use tracing::{debug, info};

pub mod agents;
pub mod providers;

use agents::chat::ChatAgent;
use agents::{Agent, AgentConfig};

#[derive(Parser, Debug)]
#[command(
    name = "ally",
    about = "An AI chat agent built with Rust and the Rig framework",
    long_about = "Ally is a command-line AI chat agent that supports multiple LLM providers including Ollama and OpenRouter. \
                  It provides an interactive chat interface similar to the original Go implementation."
)]
struct Args {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// LLM provider to use (ollama or openrouter)
    #[arg(short, long, default_value = "ollama")]
    provider: String,

    /// Model name to use
    #[arg(short, long, default_value = "llama3.2")]
    model: String,

    /// OpenRouter API key (required if using openrouter provider)
    /// Can also be set via OPENROUTER_API_KEY environment variable
    #[arg(long, env)]
    openrouter_api_key: Option<String>,
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

    // Create agent configuration
    let config = AgentConfig::new(
        args.verbose,
        args.provider,
        args.model,
        args.openrouter_api_key,
    );

    // Create and run the chat agent
    let agent = ChatAgent::new(config)?;
    agent.run().await?;

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
