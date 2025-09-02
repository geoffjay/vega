use anyhow::Result;
use clap::Parser;
use std::env;
use std::path::PathBuf;
// Main module - uses custom logger for all output
use uuid::Uuid;

pub mod acp;
pub mod agent_instructions;
pub mod agents;
pub mod context;
pub mod embeddings;
pub mod input;
pub mod logging;
pub mod providers;
pub mod tools;
pub mod web;

use crate::agent_instructions::AgentInstructionLoader;
use crate::web::start_web_server_with_logger;
use agents::chat::ChatAgent;
use agents::{Agent, AgentConfig};
use context::ContextStore;
use logging::{LogLevel, Logger, LoggerConfig};

#[derive(Parser, Debug)]
#[command(
    name = "vega",
    about = "An AI chat agent built with Rust and the Rig framework",
    long_about = "Vega is a command-line AI chat agent that supports multiple LLM providers including Ollama and OpenRouter. \
                  It provides an interactive chat interface with persistent context across sessions.\n\n\
                  Environment Variables:\n\
                  - VEGA_PROVIDER: Set the LLM provider (ollama, openrouter)\n\
                  - VEGA_MODEL: Set the model name\n\
                  - VEGA_EMBEDDING_PROVIDER: Set the embedding provider (openai, ollama, simple)\n\
                  - VEGA_EMBEDDING_MODEL: Set the embedding model name\n\
                  - VEGA_CONTEXT_DB: Set the context database path\n\
                  - VEGA_SESSION_ID: Set the session ID for context sharing\n\
                  - VEGA_LOG_OUTPUT: Set log output destinations (console, file, vector)\n\
                  - VEGA_LOG_FILE: Set the log file path\n\
                  - VEGA_LOG_STRUCTURED: Enable structured JSON logging\n\
                  - VEGA_LOG_LEVEL: Set log level (error, warn, info, debug, trace)\n\
                  - VEGA_COMMAND_HISTORY_LENGTH: Set command history length (default: 100)\n\
                  - OPENROUTER_API_KEY: Set the OpenRouter API key\n\
                  - ANTHROPIC_API_KEY: Set the Anthropic API key\n\
                  - OPENAI_API_KEY: Set the OpenAI API key for embeddings"
)]
struct Args {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// LLM provider to use (ollama or openrouter)
    /// Can also be set via VEGA_PROVIDER environment variable
    #[arg(short, long, env = "VEGA_PROVIDER", default_value = "ollama")]
    provider: String,

    /// Model name to use
    /// Can also be set via VEGA_MODEL environment variable
    #[arg(short, long, env = "VEGA_MODEL", default_value = "llama3.2")]
    model: String,

    /// OpenRouter API key (required if using openrouter provider)
    /// Can also be set via OPENROUTER_API_KEY environment variable
    #[arg(long, env)]
    openrouter_api_key: Option<String>,

    /// Anthropic API key (required if using anthropic provider)
    /// Can also be set via ANTHROPIC_API_KEY environment variable
    #[arg(long, env)]
    anthropic_api_key: Option<String>,

    /// Embedding provider to use (openai, ollama, or simple)
    /// Can also be set via VEGA_EMBEDDING_PROVIDER environment variable
    #[arg(long, env = "VEGA_EMBEDDING_PROVIDER", default_value = "simple")]
    embedding_provider: String,

    /// Embedding model name to use
    /// Can also be set via VEGA_EMBEDDING_MODEL environment variable
    #[arg(long, env = "VEGA_EMBEDDING_MODEL")]
    embedding_model: Option<String>,

    /// OpenAI API key (required if using openai embedding provider)
    /// Can also be set via OPENAI_API_KEY environment variable
    #[arg(long, env)]
    openai_api_key: Option<String>,

    /// Path to the context database file
    /// Can also be set via VEGA_CONTEXT_DB environment variable
    #[arg(long, env = "VEGA_CONTEXT_DB", default_value = "./vega_context.db")]
    context_db: PathBuf,

    /// Session ID for context sharing (generates new if not provided)
    /// Can also be set via VEGA_SESSION_ID environment variable
    #[arg(long, env = "VEGA_SESSION_ID")]
    session_id: Option<String>,

    /// Port for the web server (default: 3000)
    #[arg(long, default_value = "3000")]
    web_port: u16,

    /// Skip tool execution confirmation prompts (YOLO mode)
    #[arg(long)]
    yolo: bool,

    /// Log output destination (console, file, vector, or combinations like "console,file")
    /// Can also be set via VEGA_LOG_OUTPUT environment variable
    #[arg(long, env = "VEGA_LOG_OUTPUT", default_value = "console")]
    log_output: String,

    /// Log file path (required if file logging is enabled)
    /// Can also be set via VEGA_LOG_FILE environment variable
    #[arg(long, env = "VEGA_LOG_FILE")]
    log_file: Option<PathBuf>,

    /// Enable structured logging (JSON format for file and vector outputs)
    /// Can also be set via VEGA_LOG_STRUCTURED environment variable
    #[arg(long, env = "VEGA_LOG_STRUCTURED")]
    log_structured: bool,

    /// Run in Agent Client Protocol (ACP) mode for editor integration
    #[arg(long)]
    acp: bool,

    /// Command history length (default: 100)
    /// Can also be set via VEGA_COMMAND_HISTORY_LENGTH environment variable
    #[arg(long, env = "VEGA_COMMAND_HISTORY_LENGTH", default_value = "100")]
    command_history_length: usize,
}

impl Args {
    /// Display configuration values from command line arguments and environment variables
    fn display_configuration(&self) {
        println!("🚀 Vega Configuration");
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
        println!("  • Log output: {}", self.log_output);
        if let Some(ref log_file) = self.log_file {
            println!("  • Log file: {}", log_file.display());
        } else {
            println!("  • Log file: <not set>");
        }
        println!(
            "  • Structured logging: {}",
            if self.log_structured {
                "enabled"
            } else {
                "disabled"
            }
        );
        println!(
            "  • ACP mode: {}",
            if self.acp { "enabled" } else { "disabled" }
        );
        println!(
            "  • Command history length: {}",
            self.command_history_length
        );

        // Display API key status (without revealing the actual keys)
        if self.openrouter_api_key.is_some() {
            println!("  • OpenRouter API key: ✓ configured");
        } else {
            println!("  • OpenRouter API key: ✗ not set");
        }

        if self.anthropic_api_key.is_some() {
            println!("  • Anthropic API key: ✓ configured");
        } else {
            println!("  • Anthropic API key: ✗ not set");
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
            ("VEGA_PROVIDER", "LLM provider"),
            ("VEGA_MODEL", "LLM model"),
            ("VEGA_EMBEDDING_PROVIDER", "Embedding provider"),
            ("VEGA_EMBEDDING_MODEL", "Embedding model"),
            ("VEGA_CONTEXT_DB", "Context database path"),
            ("VEGA_SESSION_ID", "Session ID"),
            ("VEGA_LOG_OUTPUT", "Log output destinations"),
            ("VEGA_LOG_FILE", "Log file path"),
            ("VEGA_LOG_STRUCTURED", "Structured logging"),
            ("VEGA_LOG_LEVEL", "Log level"),
            ("VEGA_COMMAND_HISTORY_LENGTH", "Command history length"),
            ("OPENROUTER_API_KEY", "OpenRouter API key"),
            ("ANTHROPIC_API_KEY", "Anthropic API key"),
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

    // Initialize tracing based on log output configuration
    let log_outputs: Vec<&str> = args.log_output.split(',').collect();
    let should_log_to_console = log_outputs.contains(&"console");

    if should_log_to_console {
        // Only initialize console tracing if console output is requested
        let filter = if args.verbose { "debug" } else { "info" };
        tracing_subscriber::fmt().with_env_filter(filter).init();
    } else {
        // Initialize a no-op subscriber to suppress tracing output
        use tracing_subscriber::filter::LevelFilter;
        use tracing_subscriber::prelude::*;
        tracing_subscriber::registry().with(LevelFilter::OFF).init();
    }

    // Generate or use provided session ID
    let is_new_session = args.session_id.is_none();
    let session_id = args
        .session_id
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // We'll log this information with our custom logger after it's initialized

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

    // Initialize custom logger
    let base_log_level = if args.verbose {
        LogLevel::Debug
    } else {
        LogLevel::Info
    };

    // Use environment variable for log level if set, with precedence for trace level
    let log_level = LogLevel::from_env_or_default(base_log_level);
    let final_log_level = if log_level == LogLevel::Trace {
        log_level // Environment variable takes precedence for trace level
    } else if args.verbose {
        LogLevel::Debug // --verbose still works for debug level
    } else {
        log_level
    };

    let mut logger_config = LoggerConfig::new(session_id.clone())
        .with_console_level(final_log_level)
        .with_structured(args.log_structured)
        .with_console_output(log_outputs.contains(&"console"));

    // Configure file logging if requested
    if log_outputs.contains(&"file") {
        if let Some(ref log_file) = args.log_file {
            logger_config = logger_config.with_file_path(Some(log_file.clone()));
        } else {
            return Err(anyhow::anyhow!(
                "File logging requested but no log file path provided. Use --log-file or VEGA_LOG_FILE."
            ));
        }
    }

    // Configure vector store logging if requested
    if log_outputs.contains(&"vector") {
        logger_config = logger_config.with_vector_store(true);
    }

    let mut logger = Logger::new(logger_config)?;

    // Add context store and embedding service for vector logging
    if log_outputs.contains(&"vector") {
        let embedding_service = std::sync::Arc::new(embedding_provider.create_service());
        logger = logger
            .with_context_store(context_arc.clone())
            .with_embedding_service(embedding_service);
    }

    let logger = std::sync::Arc::new(logger);

    // Log startup information with custom logger
    if args.verbose {
        logger.info("Verbose logging enabled".to_string()).await?;
        logger
            .debug("Verbose logging enabled with custom logger".to_string())
            .await?;
    }

    logger
        .info(format!("Context database: {:?}", args.context_db))
        .await?;
    if is_new_session {
        logger
            .info(format!("Generated new session ID: {}", session_id))
            .await?;
    } else {
        logger
            .info(format!("Using existing session ID: {}", session_id))
            .await?;
    }

    // Discover and load agent instructions
    let instruction_loader = AgentInstructionLoader::new()?;
    let agent_instructions = match instruction_loader.discover_instructions()? {
        Some(instructions) => {
            logger
                .info(format!(
                    "Loaded agent instructions from: {} ({})",
                    instructions.source_path.display(),
                    instructions.file_type.filename()
                ))
                .await?;
            Some(instructions)
        }
        None => {
            logger
                .debug("No AGENTS.md or VEGA.md files found in directory tree".to_string())
                .await?;
            None
        }
    };

    // Create agent configuration
    let api_key = match args.provider.as_str() {
        "openrouter" => args.openrouter_api_key,
        "anthropic" => args.anthropic_api_key,
        _ => None,
    };

    let mut config = AgentConfig::new(
        args.verbose,
        args.provider,
        args.model,
        api_key,
        args.embedding_provider,
        args.embedding_model,
        args.openai_api_key,
        args.yolo,
    );

    // Add agent instructions if found
    if let Some(instructions) = agent_instructions {
        config = config.with_instructions(instructions);
    }

    // Check if running in ACP mode
    if args.acp {
        logger
            .info("Starting Vega in Agent Client Protocol (ACP) mode".to_string())
            .await?;

        // Run the ACP server
        return crate::acp::start_acp_server(config, context_arc, logger).await;
    }

    // Start web server in background
    let web_context = context_arc.clone();
    let web_logger = logger.clone();
    let web_port = args.web_port;
    tokio::spawn(async move {
        if let Err(e) = start_web_server_with_logger(web_context, Some(web_logger), web_port).await
        {
            eprintln!("Web server error: {}", e);
        }
    });

    logger
        .info(format!(
            "Web interface available at http://127.0.0.1:{}",
            args.web_port
        ))
        .await?;

    // Create the chat agent
    let agent = ChatAgent::new(config)?.with_logger(logger.clone());

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
            std::env::remove_var("VEGA_PROVIDER");
            std::env::remove_var("VEGA_MODEL");
            std::env::remove_var("OPENROUTER_API_KEY");
        }

        let args = Args::try_parse_from(&["vega"]).unwrap();

        assert_eq!(args.verbose, false);
        assert_eq!(args.provider, "ollama");
        assert_eq!(args.model, "llama3.2");
        assert_eq!(args.openrouter_api_key, None);
        assert_eq!(args.yolo, false);
    }

    #[test]
    fn test_verbose_flag() {
        let args = Args::try_parse_from(&["vega", "--verbose"]).unwrap();
        assert_eq!(args.verbose, true);

        let args = Args::try_parse_from(&["vega", "-v"]).unwrap();
        assert_eq!(args.verbose, true);
    }

    #[test]
    fn test_provider_option() {
        let args = Args::try_parse_from(&["vega", "--provider", "openrouter"]).unwrap();
        assert_eq!(args.provider, "openrouter");

        let args = Args::try_parse_from(&["vega", "-p", "ollama"]).unwrap();
        assert_eq!(args.provider, "ollama");
    }

    #[test]
    fn test_model_option() {
        let args = Args::try_parse_from(&["vega", "--model", "gpt-4"]).unwrap();
        assert_eq!(args.model, "gpt-4");

        let args = Args::try_parse_from(&["vega", "-m", "llama3.1"]).unwrap();
        assert_eq!(args.model, "llama3.1");
    }

    #[test]
    fn test_openrouter_api_key() {
        let args = Args::try_parse_from(&["vega", "--openrouter-api-key", "test-key"]).unwrap();
        assert_eq!(args.openrouter_api_key, Some("test-key".to_string()));
    }

    #[test]
    fn test_anthropic_api_key() {
        let args = Args::try_parse_from(&["vega", "--anthropic-api-key", "test-key"]).unwrap();
        assert_eq!(args.anthropic_api_key, Some("test-key".to_string()));
    }

    #[test]
    fn test_combined_args() {
        let args = Args::try_parse_from(&[
            "vega",
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
            anthropic_api_key: None,
            embedding_provider: "simple".to_string(),
            embedding_model: None,
            openai_api_key: None,
            context_db: "./test_context.db".into(),
            session_id: Some("test_session".to_string()),
            web_port: 3000,
            yolo: false,
            log_output: "console".to_string(),
            log_file: None,
            log_structured: false,
            acp: false,
            command_history_length: 100,
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
        assert!(config.agent_instructions.is_none());
    }
}
