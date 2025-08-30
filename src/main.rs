use anyhow::Result;
use clap::Parser;
use rig::{client::CompletionClient, completion::Prompt, providers};
use std::io::{self, Write};
use tracing::{debug, error, info};

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

enum LLMProvider {
    Ollama {
        client: providers::ollama::Client,
        model: String,
    },
    OpenRouter {
        client: providers::openrouter::Client,
        model: String,
    },
}

struct ChatAgent {
    provider: LLMProvider,
    verbose: bool,
}

impl ChatAgent {
    fn new(args: &Args) -> Result<Self> {
        let provider = match args.provider.as_str() {
            "ollama" => {
                if args.verbose {
                    info!("Initializing Ollama client with model: {}", args.model);
                }

                let client = providers::ollama::Client::new();
                LLMProvider::Ollama {
                    client,
                    model: args.model.clone(),
                }
            }
            "openrouter" => {
                let api_key = args.openrouter_api_key.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("OpenRouter API key is required for openrouter provider. Set --openrouter-api-key or OPENROUTER_API_KEY environment variable.")
                })?;

                if args.verbose {
                    info!("Initializing OpenRouter client with model: {}", args.model);
                }

                let client = providers::openrouter::Client::new(api_key);
                LLMProvider::OpenRouter {
                    client,
                    model: args.model.clone(),
                }
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported provider: {}. Supported providers: ollama, openrouter",
                    args.provider
                ));
            }
        };

        Ok(ChatAgent {
            provider,
            verbose: args.verbose,
        })
    }

    async fn run(&self) -> Result<()> {
        if self.verbose {
            info!("Starting chat session");
        }

        println!("Chat with AI Agent (use 'quit' or Ctrl+C to exit)");
        println!("Type your message and press Enter to send.");
        println!();

        loop {
            // Get user input
            print!("\x1b[94mYou\x1b[0m: ");
            io::stdout().flush()?;

            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(0) => {
                    if self.verbose {
                        debug!("EOF received, ending chat session");
                    }
                    break;
                }
                Ok(_) => {
                    let user_input = input.trim();

                    // Check for quit commands
                    if user_input.is_empty() {
                        if self.verbose {
                            debug!("Skipping empty message");
                        }
                        continue;
                    }

                    if user_input.eq_ignore_ascii_case("quit")
                        || user_input.eq_ignore_ascii_case("exit")
                    {
                        if self.verbose {
                            info!("User requested to quit");
                        }
                        break;
                    }

                    if self.verbose {
                        debug!("User input received: {:?}", user_input);
                    }

                    // Send message to AI and get response
                    match self.get_response(user_input).await {
                        Ok(response) => {
                            println!("\x1b[93mAgent\x1b[0m: {}", response);
                            println!();
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

        if self.verbose {
            info!("Chat session ended");
        }

        Ok(())
    }

    async fn get_response(&self, prompt: &str) -> Result<String> {
        if self.verbose {
            debug!("Sending prompt to AI model");
        }

        let preamble =
            "You are a helpful AI assistant. Respond in a conversational and helpful manner.";

        let response = match &self.provider {
            LLMProvider::Ollama { client, model } => {
                let agent = client
                    .agent(model)
                    .preamble(preamble)
                    .max_tokens(2048)
                    .build();
                agent.prompt(prompt).await?
            }
            LLMProvider::OpenRouter { client, model } => {
                let agent = client
                    .agent(model)
                    .preamble(preamble)
                    .max_tokens(2048)
                    .build();
                agent.prompt(prompt).await?
            }
        };

        if self.verbose {
            debug!("Received response from AI model");
        }

        Ok(response)
    }
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

    // Create and run the agent
    let agent = ChatAgent::new(&args)?;
    agent.run().await?;

    Ok(())
}
