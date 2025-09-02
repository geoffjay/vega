//! Agent Client Protocol (ACP) implementation for Ally
//!
//! This module provides ACP server functionality, allowing Ally to be used
//! as an agent in ACP-compatible editors like Zed.

use agent_client_protocol::{self as acp, Client};
use anyhow::Result;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tracing::{debug, error, info, warn};

use crate::agents::AgentConfig;
use crate::agents::chat::ChatAgent;
use crate::context::ContextStore;
use crate::logging::Logger;

/// ACP Agent implementation for Vega
pub struct AcpAgent {
    /// Configuration for the underlying Vega agent
    config: AgentConfig,
    /// Context store for conversation history
    context_store: Arc<ContextStore>,
    /// Logger for ACP operations
    logger: Arc<Logger>,
    /// Channel for sending session updates to the client
    session_update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
    /// Counter for generating session IDs
    next_session_id: AtomicU64,
    /// Current working directory
    cwd: Arc<Mutex<PathBuf>>,
}

impl AcpAgent {
    /// Create a new ACP agent instance
    pub fn new(
        config: AgentConfig,
        context_store: Arc<ContextStore>,
        logger: Arc<Logger>,
        session_update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
    ) -> Self {
        Self {
            config,
            context_store,
            logger,
            session_update_tx,
            next_session_id: AtomicU64::new(0),
            cwd: Arc::new(Mutex::new(std::env::current_dir().unwrap_or_default())),
        }
    }

    /// Send a session notification to the client
    async fn send_session_update(
        &self,
        session_id: &acp::SessionId,
        update: acp::SessionUpdate,
    ) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.session_update_tx
            .send((
                acp::SessionNotification {
                    session_id: session_id.clone(),
                    update,
                },
                tx,
            ))
            .map_err(|_| anyhow::anyhow!("Failed to send session update"))?;

        rx.await
            .map_err(|_| anyhow::anyhow!("Failed to receive session update confirmation"))?;

        Ok(())
    }

    /// Send a text message chunk to the client
    async fn send_message_chunk(&self, session_id: &acp::SessionId, text: &str) -> Result<()> {
        let content = acp::ContentBlock::Text(acp::TextContent {
            text: text.to_string().into(),
            annotations: None,
        });

        self.send_session_update(
            session_id,
            acp::SessionUpdate::AgentMessageChunk { content },
        )
        .await
    }

    /// Process a prompt using the underlying Ally chat agent
    async fn process_prompt(&self, session_id: &acp::SessionId, prompt: &str) -> Result<()> {
        // Create a chat agent for this session (we don't store them as they're stateless)
        let chat_agent = ChatAgent::new(self.config.clone())?.with_logger(self.logger.clone());

        // Log the prompt processing
        self.logger
            .info(format!(
                "Processing ACP prompt for session {}: {}",
                session_id.0, prompt
            ))
            .await?;

        // Send the prompt to the chat agent and stream the response
        // For now, we'll use a simplified approach - in a full implementation,
        // we'd want to stream the response as it's generated
        match self
            .get_agent_response(&chat_agent, prompt, &session_id.0.to_string())
            .await
        {
            Ok(response) => {
                // Send the response as message chunks
                self.send_message_chunk(session_id, &response).await?;
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Error processing prompt: {}", e);
                self.logger.error(error_msg.clone()).await?;
                self.send_message_chunk(session_id, &error_msg).await?;
                Err(e)
            }
        }
    }

    /// Get a response from the chat agent
    async fn get_agent_response(
        &self,
        _chat_agent: &ChatAgent,
        prompt: &str,
        session_id: &str,
    ) -> Result<String> {
        // This is a simplified version - in the full implementation,
        // we'd need to integrate more deeply with the ChatAgent's response generation
        // For now, we'll create a basic response

        // Store user input in context
        use crate::context::ContextEntry;
        let user_entry = ContextEntry::new(
            "acp".to_string(),
            session_id.to_string(),
            prompt.to_string(),
            "user".to_string(),
        );

        // Create a simple embedding for the prompt (this would use the actual embedding service)
        let embedding = vec![0.0; 1536]; // Placeholder embedding
        if let Err(e) = self
            .context_store
            .store_context(user_entry, embedding)
            .await
        {
            warn!("Failed to store user context: {}", e);
        }

        // Generate a response (this is simplified - the real implementation would
        // use the full ChatAgent functionality)
        let response = format!(
            "I received your message: \"{}\". This is a response from Ally via the Agent Client Protocol. \
            The full integration with Ally's chat capabilities is in progress.",
            prompt
        );

        // Store agent response in context
        let agent_entry = ContextEntry::new(
            "acp".to_string(),
            session_id.to_string(),
            response.clone(),
            "assistant".to_string(),
        );

        let response_embedding = vec![0.0; 1536]; // Placeholder embedding
        if let Err(e) = self
            .context_store
            .store_context(agent_entry, response_embedding)
            .await
        {
            warn!("Failed to store agent context: {}", e);
        }

        Ok(response)
    }
}

impl acp::Agent for AcpAgent {
    async fn initialize(
        &self,
        arguments: acp::InitializeRequest,
    ) -> Result<acp::InitializeResponse, acp::Error> {
        info!("ACP Initialize request received: {:?}", arguments);

        self.logger
            .info(format!(
                "ACP agent initialized with protocol version: {:?}",
                arguments.protocol_version
            ))
            .await
            .map_err(|_| acp::Error::internal_error())?;

        Ok(acp::InitializeResponse {
            protocol_version: acp::V1,
            agent_capabilities: acp::AgentCapabilities {
                load_session: false,
                prompt_capabilities: acp::PromptCapabilities {
                    image: false,
                    audio: false,
                    embedded_context: false,
                },
            },
            auth_methods: Vec::new(),
        })
    }

    async fn authenticate(&self, arguments: acp::AuthenticateRequest) -> Result<(), acp::Error> {
        info!("ACP Authenticate request received: {:?}", arguments);

        self.logger
            .info("ACP authentication completed (no auth required)".to_string())
            .await
            .map_err(|_| acp::Error::internal_error())?;

        Ok(())
    }

    async fn new_session(
        &self,
        arguments: acp::NewSessionRequest,
    ) -> Result<acp::NewSessionResponse, acp::Error> {
        info!("ACP New session request received: {:?}", arguments);

        let session_id = self.next_session_id.fetch_add(1, Ordering::SeqCst);
        let session_id_str = format!("acp-{}", session_id);

        // Update working directory if provided
        let mut current_cwd = self.cwd.lock().await;
        *current_cwd = arguments.cwd;

        self.logger
            .info(format!("Created new ACP session: {}", session_id_str))
            .await
            .map_err(|_| acp::Error::internal_error())?;

        Ok(acp::NewSessionResponse {
            session_id: acp::SessionId(session_id_str.into()),
        })
    }

    async fn load_session(&self, arguments: acp::LoadSessionRequest) -> Result<(), acp::Error> {
        info!("ACP Load session request received: {:?}", arguments);

        // For now, we don't support loading existing sessions
        // This could be implemented to restore conversation history
        Err(acp::Error::method_not_found())
    }

    async fn prompt(
        &self,
        arguments: acp::PromptRequest,
    ) -> Result<acp::PromptResponse, acp::Error> {
        info!(
            "ACP Prompt request received for session: {:?}",
            arguments.session_id
        );

        // Convert the prompt content to a string
        let mut prompt_text = String::new();
        for content in &arguments.prompt {
            match content {
                acp::ContentBlock::Text(text_content) => {
                    prompt_text.push_str(&text_content.text);
                    prompt_text.push(' ');
                }
                acp::ContentBlock::Image(_) => {
                    prompt_text.push_str("[Image content] ");
                }
                acp::ContentBlock::Audio(_) => {
                    prompt_text.push_str("[Audio content] ");
                }
                acp::ContentBlock::ResourceLink(resource_link) => {
                    prompt_text.push_str(&format!("[Resource: {}] ", resource_link.uri));
                }
                acp::ContentBlock::Resource(_) => {
                    prompt_text.push_str("[Resource content] ");
                }
            }
        }

        // Process the prompt
        if let Err(e) = self
            .process_prompt(&arguments.session_id, &prompt_text.trim())
            .await
        {
            error!("Failed to process prompt: {}", e);
            return Err(acp::Error::internal_error());
        }

        Ok(acp::PromptResponse {
            stop_reason: acp::StopReason::EndTurn,
        })
    }

    async fn cancel(&self, args: acp::CancelNotification) -> Result<(), acp::Error> {
        info!("ACP Cancel request received: {:?}", args);

        self.logger
            .info(format!(
                "ACP operation cancelled for session: {:?}",
                args.session_id
            ))
            .await
            .map_err(|_| acp::Error::internal_error())?;

        Ok(())
    }
}

/// ACP Client implementation for handling client-side operations
pub struct AllyAcpClient {
    /// Logger for client operations
    logger: Arc<Logger>,
    /// Current working directory
    cwd: Arc<Mutex<PathBuf>>,
}

impl AllyAcpClient {
    pub fn new(logger: Arc<Logger>) -> Self {
        Self {
            logger,
            cwd: Arc::new(Mutex::new(std::env::current_dir().unwrap_or_default())),
        }
    }
}

impl acp::Client for AllyAcpClient {
    async fn request_permission(
        &self,
        _args: acp::RequestPermissionRequest,
    ) -> Result<acp::RequestPermissionResponse, acp::Error> {
        // For now, we'll deny all permission requests
        // This could be extended to show prompts to the user
        Err(acp::Error::method_not_found())
    }

    async fn write_text_file(&self, args: acp::WriteTextFileRequest) -> Result<(), acp::Error> {
        self.logger
            .info(format!("ACP write file request: {:?}", args.path))
            .await
            .map_err(|_| acp::Error::internal_error())?;

        // Resolve the path relative to current working directory
        let cwd = self.cwd.lock().await;
        let full_path = if args.path.is_absolute() {
            args.path
        } else {
            cwd.join(&args.path)
        };

        // Write the file
        tokio::fs::write(&full_path, &args.content)
            .await
            .map_err(|_e| acp::Error::internal_error())?;

        self.logger
            .info(format!("Successfully wrote file: {:?}", full_path))
            .await
            .map_err(|_| acp::Error::internal_error())?;

        Ok(())
    }

    async fn read_text_file(
        &self,
        args: acp::ReadTextFileRequest,
    ) -> Result<acp::ReadTextFileResponse, acp::Error> {
        self.logger
            .info(format!("ACP read file request: {:?}", args.path))
            .await
            .map_err(|_| acp::Error::internal_error())?;

        // Resolve the path relative to current working directory
        let cwd = self.cwd.lock().await;
        let full_path = if args.path.is_absolute() {
            args.path
        } else {
            cwd.join(&args.path)
        };

        // Read the file
        let content = tokio::fs::read_to_string(&full_path)
            .await
            .map_err(|_e| acp::Error::internal_error())?;

        self.logger
            .info(format!("Successfully read file: {:?}", full_path))
            .await
            .map_err(|_| acp::Error::internal_error())?;

        Ok(acp::ReadTextFileResponse { content })
    }

    async fn session_notification(&self, args: acp::SessionNotification) -> Result<(), acp::Error> {
        // Handle session notifications from the agent
        match args.update {
            acp::SessionUpdate::AgentMessageChunk { content } => {
                let text = match content {
                    acp::ContentBlock::Text(text_content) => text_content.text.to_string(),
                    acp::ContentBlock::Image(_) => "[Image]".to_string(),
                    acp::ContentBlock::Audio(_) => "[Audio]".to_string(),
                    acp::ContentBlock::ResourceLink(resource_link) => resource_link.uri,
                    acp::ContentBlock::Resource(_) => "[Resource]".to_string(),
                };

                // In a real client, this would be displayed to the user
                println!("Agent: {}", text);
            }
            _ => {
                // Handle other types of session updates
                debug!("Received session update: {:?}", args.update);
            }
        }

        Ok(())
    }
}

/// Start the ACP server
pub async fn start_acp_server(
    config: AgentConfig,
    context_store: Arc<ContextStore>,
    logger: Arc<Logger>,
) -> Result<()> {
    info!("Starting ACP server on stdio");

    let outgoing = tokio::io::stdout().compat_write();
    let incoming = tokio::io::stdin().compat();

    // Create channels for session updates
    let (session_update_tx, mut session_update_rx) = mpsc::unbounded_channel();

    // Create the ACP agent
    let agent = AcpAgent::new(config, context_store, logger.clone(), session_update_tx);

    // Use LocalSet for non-Send futures
    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async move {
            // Create the ACP connection
            let (conn, handle_io) =
                acp::AgentSideConnection::new(agent, outgoing, incoming, |fut| {
                    tokio::task::spawn_local(fut);
                });

            // Handle session notifications
            tokio::task::spawn_local(async move {
                while let Some((session_notification, tx)) = session_update_rx.recv().await {
                    let result = conn.session_notification(session_notification).await;
                    if let Err(e) = result {
                        error!("Failed to send session notification: {}", e);
                        break;
                    }
                    tx.send(()).ok();
                }
            });

            // Run the I/O handler
            handle_io.await
        })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::AgentConfig;
    use crate::logging::{LogLevel, LoggerConfig};
    use agent_client_protocol::Agent;
    use tempfile::TempDir;

    fn create_test_config() -> AgentConfig {
        AgentConfig::new(
            false,
            "ollama".to_string(),
            "llama3.2".to_string(),
            None,
            "simple".to_string(),
            None,
            None,
            false,
        )
    }

    async fn create_test_context_store() -> Result<Arc<ContextStore>> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let context = ContextStore::new(&db_path, 1536).await?;
        Ok(Arc::new(context))
    }

    async fn create_test_logger() -> Result<Arc<Logger>> {
        let logger_config = LoggerConfig::new("test-session".to_string())
            .with_console_level(LogLevel::Info)
            .with_console_output(false);
        let logger = Logger::new(logger_config)?;
        Ok(Arc::new(logger))
    }

    #[tokio::test]
    async fn test_acp_agent_creation() -> Result<()> {
        let config = create_test_config();
        let context_store = create_test_context_store().await?;
        let logger = create_test_logger().await?;
        let (tx, _rx) = mpsc::unbounded_channel();

        let agent = AcpAgent::new(config, context_store, logger, tx);

        // Test initialization
        let init_request = acp::InitializeRequest {
            protocol_version: acp::V1,
            client_capabilities: acp::ClientCapabilities::default(),
        };

        let response = agent.initialize(init_request).await?;
        assert_eq!(response.protocol_version, acp::V1);
        assert_eq!(response.agent_capabilities.load_session, false);

        Ok(())
    }

    #[tokio::test]
    async fn test_acp_client_creation() -> Result<()> {
        let logger = create_test_logger().await?;
        let _client = AllyAcpClient::new(logger);
        Ok(())
    }
}
