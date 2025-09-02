use anyhow::Result;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use std::env;
use tracing::{debug, warn};

use crate::context::ContextStore;

/// Input handler that provides command history and line editing capabilities
pub struct InputHandler {
    editor: DefaultEditor,
    session_id: String,
    context_store: std::sync::Arc<ContextStore>,
    history_length: usize,
}

impl InputHandler {
    /// Create a new input handler
    pub fn new(
        session_id: String,
        context_store: std::sync::Arc<ContextStore>,
        history_length: Option<usize>,
    ) -> Result<Self> {
        let editor = DefaultEditor::new()?;

        // Get history length from parameter or environment variable or default
        let history_length = history_length
            .or_else(|| {
                env::var("ALLY_COMMAND_HISTORY_LENGTH")
                    .ok()
                    .and_then(|s| s.parse().ok())
            })
            .unwrap_or(100);

        debug!(
            "Initialized input handler with history length: {}",
            history_length
        );

        Ok(Self {
            editor,
            session_id,
            context_store,
            history_length,
        })
    }

    /// Load command history from the database
    pub async fn load_history(&mut self) -> Result<()> {
        let commands = self
            .context_store
            .get_command_history(&self.session_id, Some(self.history_length))
            .await?;

        // Add commands to rustyline history in reverse order (oldest first)
        for command in commands.iter().rev() {
            if let Err(e) = self.editor.add_history_entry(command) {
                warn!("Failed to add command to history: {}", e);
            }
        }

        debug!("Loaded {} commands from history", commands.len());
        Ok(())
    }

    /// Read a line of input with history and editing support
    pub async fn read_line(&mut self, prompt: &str) -> Result<Option<String>> {
        match self.editor.readline(prompt) {
            Ok(line) => {
                let trimmed = line.trim();

                // Don't store empty lines
                if !trimmed.is_empty() {
                    // Add to rustyline history
                    if let Err(e) = self.editor.add_history_entry(&line) {
                        warn!("Failed to add command to rustyline history: {}", e);
                    }

                    // Store in database
                    if let Err(e) = self
                        .context_store
                        .store_command_history(&self.session_id, trimmed)
                        .await
                    {
                        warn!("Failed to store command in database: {}", e);
                    }

                    // Trim history if it gets too long
                    if let Err(e) = self
                        .context_store
                        .trim_command_history(&self.session_id, self.history_length)
                        .await
                    {
                        warn!("Failed to trim command history: {}", e);
                    }
                }

                Ok(Some(line))
            }
            Err(ReadlineError::Interrupted) => {
                debug!("Ctrl-C pressed");
                Ok(None)
            }
            Err(ReadlineError::Eof) => {
                debug!("EOF received");
                Ok(None)
            }
            Err(err) => {
                warn!("Error reading input: {}", err);
                Err(err.into())
            }
        }
    }

    /// Clear the command history
    pub async fn clear_history(&mut self) -> Result<()> {
        self.editor.clear_history()?;
        self.context_store
            .clear_command_history(&self.session_id)
            .await?;
        debug!("Cleared command history");
        Ok(())
    }
}
