use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use tokio::time::{Duration, Instant};

/// Progress phases for LLM operations
#[derive(Debug, Clone)]
pub enum ProgressPhase {
    Preparing,
    Embedding,
    ContextRetrieval,
    Thinking,
    ToolExecution(String),
    Finalizing,
}

impl ProgressPhase {
    pub fn emoji(&self) -> &'static str {
        match self {
            ProgressPhase::Preparing => "⚙️",
            ProgressPhase::Embedding => "🔍",
            ProgressPhase::ContextRetrieval => "📚",
            ProgressPhase::Thinking => "🧠",
            ProgressPhase::ToolExecution(_) => "🔧",
            ProgressPhase::Finalizing => "✨",
        }
    }

    pub fn message(&self) -> String {
        match self {
            ProgressPhase::Preparing => "Preparing".to_string(),
            ProgressPhase::Embedding => "Generating embeddings".to_string(),
            ProgressPhase::ContextRetrieval => "Retrieving context".to_string(),
            ProgressPhase::Thinking => "Thinking".to_string(),
            ProgressPhase::ToolExecution(tool) => format!("Using {}", tool),
            ProgressPhase::Finalizing => "Finalizing response".to_string(),
        }
    }
}

/// Progress update message
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    pub phase: ProgressPhase,
    pub message: Option<String>,
}

/// Streaming progress indicator for LLM operations
pub struct StreamingProgress {
    sender: broadcast::Sender<ProgressUpdate>,
    current_phase: Arc<Mutex<Option<ProgressPhase>>>,
    start_time: Instant,
}

impl StreamingProgress {
    /// Create a new streaming progress indicator
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(100);
        Self {
            sender,
            current_phase: Arc::new(Mutex::new(None)),
            start_time: Instant::now(),
        }
    }

    /// Update the current progress phase
    pub async fn update_phase(&self, phase: ProgressPhase, message: Option<String>) {
        *self.current_phase.lock().await = Some(phase.clone());
        let _ = self.sender.send(ProgressUpdate { phase, message });
    }

    /// Start the visual progress indicator
    pub async fn start_indicator(&self) -> tokio::task::JoinHandle<()> {
        let mut receiver = self.sender.subscribe();
        let start_time = self.start_time;

        tokio::spawn(async move {
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let mut frame_index = 0;
            let mut current_display_phase = ProgressPhase::Preparing;
            let mut custom_message: Option<String> = None;

            loop {
                // Check for phase updates
                if let Ok(update) = receiver.try_recv() {
                    current_display_phase = update.phase;
                    custom_message = update.message;
                }

                let elapsed = start_time.elapsed();
                let elapsed_str = if elapsed.as_secs() > 0 {
                    format!(" ({}s)", elapsed.as_secs())
                } else {
                    String::new()
                };

                let default_message = current_display_phase.message();
                let message = custom_message.as_ref().unwrap_or(&default_message);

                print!(
                    "\r\x1b[93m{}\x1b[0m {} {}{}...",
                    frames[frame_index],
                    current_display_phase.emoji(),
                    message,
                    elapsed_str
                );
                io::stdout().flush().unwrap();

                frame_index = (frame_index + 1) % frames.len();

                // Check if we should stop (no more phases)
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_millis(100)) => {},
                    _ = receiver.recv() => {
                        // Continue with new update
                    }
                }
            }
        })
    }

    /// Stop the progress indicator and clear the line
    pub fn stop(&self) {
        print!("\r\x1b[K"); // Clear the current line
        io::stdout().flush().unwrap();
    }
}

impl Default for StreamingProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to show a simple progress indicator
pub async fn show_simple_progress(message: &str, emoji: &str) -> tokio::task::JoinHandle<()> {
    let message = message.to_string();
    let emoji = emoji.to_string();

    tokio::spawn(async move {
        let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let mut frame_index = 0;
        let start_time = Instant::now();

        loop {
            let elapsed = start_time.elapsed();
            let elapsed_str = if elapsed.as_secs() > 0 {
                format!(" ({}s)", elapsed.as_secs())
            } else {
                String::new()
            };

            print!(
                "\r\x1b[93m{}\x1b[0m {} {}{}...",
                frames[frame_index], emoji, message, elapsed_str
            );
            io::stdout().flush().unwrap();

            frame_index = (frame_index + 1) % frames.len();
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
}

/// Stop a progress indicator and clear the line
pub fn stop_progress() {
    print!("\r\x1b[K"); // Clear the current line
    io::stdout().flush().unwrap();
}
