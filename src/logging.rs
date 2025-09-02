use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
// Logging module - custom logger implementation
use uuid::Uuid;

use crate::context::ContextStore;
use crate::embeddings::EmbeddingService;

/// Configuration for the custom logger
#[derive(Debug, Clone)]
pub struct LoggerConfig {
    /// Log level for console output
    pub console_level: LogLevel,
    /// Whether to enable structured logging (JSON format)
    pub structured: bool,
    /// File path for file logging (if enabled)
    pub file_path: Option<PathBuf>,
    /// Whether to log to the vector store
    pub vector_store: bool,
    /// Whether to log to console
    pub console_output: bool,
    /// Session ID for this logging session
    pub session_id: String,
}

impl LoggerConfig {
    pub fn new(session_id: String) -> Self {
        Self {
            console_level: LogLevel::Info,
            structured: false,
            file_path: None,
            vector_store: false,
            console_output: true,
            session_id,
        }
    }

    pub fn with_console_level(mut self, level: LogLevel) -> Self {
        self.console_level = level;
        self
    }

    pub fn with_structured(mut self, structured: bool) -> Self {
        self.structured = structured;
        self
    }

    pub fn with_file_path(mut self, path: Option<PathBuf>) -> Self {
        self.file_path = path;
        self
    }

    pub fn with_vector_store(mut self, enabled: bool) -> Self {
        self.vector_store = enabled;
        self
    }

    pub fn with_console_output(mut self, enabled: bool) -> Self {
        self.console_output = enabled;
        self
    }
}

/// Log levels supported by the custom logger
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

impl LogLevel {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "error" => LogLevel::Error,
            "warn" | "warning" => LogLevel::Warn,
            "info" => LogLevel::Info,
            "debug" => LogLevel::Debug,
            "trace" => LogLevel::Trace,
            _ => LogLevel::Info,
        }
    }

    /// Get log level from environment variable, with fallback to provided default
    pub fn from_env_or_default(default: LogLevel) -> Self {
        if let Ok(level_str) = std::env::var("ALLY_LOG_LEVEL") {
            Self::from_str(&level_str)
        } else {
            default
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        }
    }
}

/// A structured log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
    pub session_id: String,
    pub module: Option<String>,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub target: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl LogEntry {
    pub fn new(
        level: LogLevel,
        message: String,
        session_id: String,
        module: Option<String>,
        file: Option<String>,
        line: Option<u32>,
        target: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            level: level.as_str().to_string(),
            message,
            session_id,
            module,
            file,
            line,
            target,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Format as a human-readable string for console output
    pub fn format_console(&self) -> String {
        let timestamp = self.timestamp.format("%Y-%m-%d %H:%M:%S%.3f UTC");
        let location = if let (Some(file), Some(line)) = (&self.file, &self.line) {
            format!(" [{}:{}]", file, line)
        } else if let Some(module) = &self.module {
            format!(" [{}]", module)
        } else {
            String::new()
        };

        format!(
            "{} [{}]{} {}",
            timestamp, self.level, location, self.message
        )
    }

    /// Format as JSON for structured logging
    pub fn format_json(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    /// Format for vector store storage (optimized for searchability)
    pub fn format_vector_store(&self) -> String {
        let mut parts = vec![
            format!("Level: {}", self.level),
            format!("Message: {}", self.message),
            format!("Session: {}", self.session_id),
        ];

        if let Some(module) = &self.module {
            parts.push(format!("Module: {}", module));
        }

        if let Some(target) = &self.target {
            parts.push(format!("Target: {}", target));
        }

        if !self.metadata.is_empty() {
            let metadata_str = self
                .metadata
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect::<Vec<_>>()
                .join(", ");
            parts.push(format!("Metadata: {}", metadata_str));
        }

        parts.join(" | ")
    }
}

/// Custom logger that can write to multiple destinations
pub struct AllyLogger {
    config: LoggerConfig,
    file_writer: Option<Arc<Mutex<std::fs::File>>>,
    context_store: Option<Arc<ContextStore>>,
    embedding_service: Option<Arc<EmbeddingService>>,
}

impl AllyLogger {
    /// Create a new logger with the given configuration
    pub fn new(config: LoggerConfig) -> Result<Self> {
        let file_writer = if let Some(ref file_path) = config.file_path {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(file_path)?;
            Some(Arc::new(Mutex::new(file)))
        } else {
            None
        };

        Ok(Self {
            config,
            file_writer,
            context_store: None,
            embedding_service: None,
        })
    }

    /// Set the context store for vector logging
    pub fn with_context_store(mut self, context_store: Arc<ContextStore>) -> Self {
        self.context_store = Some(context_store);
        self
    }

    /// Set the embedding service for vector logging
    pub fn with_embedding_service(mut self, embedding_service: Arc<EmbeddingService>) -> Self {
        self.embedding_service = Some(embedding_service);
        self
    }

    /// Log a message at the specified level
    pub async fn log(
        &self,
        level: LogLevel,
        message: String,
        module: Option<String>,
        file: Option<String>,
        line: Option<u32>,
        target: Option<String>,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<()> {
        // Check if we should log at this level
        if level > self.config.console_level {
            return Ok(());
        }

        let mut entry = LogEntry::new(
            level,
            message,
            self.config.session_id.clone(),
            module,
            file,
            line,
            target,
        );

        if let Some(metadata) = metadata {
            entry = entry.with_metadata(metadata);
        }

        // Log to console if enabled
        if self.config.console_output {
            self.log_to_console(&entry).await?;
        }

        // Log to file if configured
        if self.file_writer.is_some() {
            self.log_to_file(&entry).await?;
        }

        // Log to vector store if configured
        if self.config.vector_store
            && self.context_store.is_some()
            && self.embedding_service.is_some()
        {
            self.log_to_vector_store(&entry).await?;
        }

        Ok(())
    }

    /// Log to console (stdout/stderr)
    async fn log_to_console(&self, entry: &LogEntry) -> Result<()> {
        let output = if self.config.structured {
            entry.format_json()?
        } else {
            entry.format_console()
        };

        // Use stderr for errors and warnings, stdout for everything else
        match entry.level.as_str() {
            "ERROR" | "WARN" => {
                eprintln!("{}", output);
            }
            _ => {
                println!("{}", output);
            }
        }

        Ok(())
    }

    /// Log to file
    async fn log_to_file(&self, entry: &LogEntry) -> Result<()> {
        if let Some(ref file_writer) = self.file_writer {
            let output = if self.config.structured {
                format!("{}\n", entry.format_json()?)
            } else {
                format!("{}\n", entry.format_console())
            };

            if let Ok(mut file) = file_writer.lock() {
                file.write_all(output.as_bytes())?;
                file.flush()?;
            }
        }
        Ok(())
    }

    /// Log to vector store
    async fn log_to_vector_store(&self, entry: &LogEntry) -> Result<()> {
        if let (Some(context_store), Some(embedding_service)) =
            (&self.context_store, &self.embedding_service)
        {
            let content = entry.format_vector_store();
            let embedding = embedding_service.embed(&content).await?;

            let context_entry = crate::context::ContextEntry::new(
                "ally_logger".to_string(),
                entry.session_id.clone(),
                content,
                "log".to_string(),
            )
            .with_metadata({
                let mut metadata = HashMap::new();
                metadata.insert("log_level".to_string(), entry.level.clone());
                metadata.insert("log_id".to_string(), entry.id.clone());
                metadata.insert("timestamp".to_string(), entry.timestamp.to_rfc3339());

                if let Some(ref module) = entry.module {
                    metadata.insert("module".to_string(), module.clone());
                }

                if let Some(ref file) = entry.file {
                    metadata.insert("file".to_string(), file.clone());
                }

                if let Some(line) = entry.line {
                    metadata.insert("line".to_string(), line.to_string());
                }

                // Add original metadata
                for (k, v) in &entry.metadata {
                    metadata.insert(format!("meta_{}", k), v.clone());
                }

                metadata
            });

            context_store
                .store_context(context_entry, embedding)
                .await?;
        }
        Ok(())
    }

    /// Convenience methods for different log levels
    pub async fn error(&self, message: String) -> Result<()> {
        self.log(LogLevel::Error, message, None, None, None, None, None)
            .await
    }

    pub async fn warn(&self, message: String) -> Result<()> {
        self.log(LogLevel::Warn, message, None, None, None, None, None)
            .await
    }

    pub async fn info(&self, message: String) -> Result<()> {
        self.log(LogLevel::Info, message, None, None, None, None, None)
            .await
    }

    pub async fn debug(&self, message: String) -> Result<()> {
        self.log(LogLevel::Debug, message, None, None, None, None, None)
            .await
    }

    pub async fn trace(&self, message: String) -> Result<()> {
        self.log(LogLevel::Trace, message, None, None, None, None, None)
            .await
    }

    /// Get logs for a session from the vector store
    pub async fn get_session_logs(
        &self,
        session_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<LogEntry>> {
        if let Some(ref context_store) = self.context_store {
            let entries = context_store.get_session_history(session_id, limit).await?;

            let mut log_entries = Vec::new();
            for entry in entries {
                // Only include log entries (role = "log")
                if entry.role == "log" {
                    // Try to reconstruct the log entry from metadata
                    if let (Some(log_level), Some(log_id), Some(timestamp_str)) = (
                        entry.metadata.get("log_level"),
                        entry.metadata.get("log_id"),
                        entry.metadata.get("timestamp"),
                    ) {
                        if let Ok(timestamp) = DateTime::parse_from_rfc3339(timestamp_str) {
                            let mut metadata = HashMap::new();
                            for (k, v) in &entry.metadata {
                                if k.starts_with("meta_") {
                                    metadata.insert(
                                        k.strip_prefix("meta_").unwrap().to_string(),
                                        v.clone(),
                                    );
                                }
                            }

                            let log_entry = LogEntry {
                                id: log_id.clone(),
                                timestamp: timestamp.with_timezone(&Utc),
                                level: log_level.clone(),
                                message: entry
                                    .content
                                    .split(" | ")
                                    .find(|part| part.starts_with("Message: "))
                                    .map(|part| {
                                        part.strip_prefix("Message: ").unwrap_or("").to_string()
                                    })
                                    .unwrap_or_else(|| entry.content.clone()),
                                session_id: entry.session_id,
                                module: entry.metadata.get("module").cloned(),
                                file: entry.metadata.get("file").cloned(),
                                line: entry.metadata.get("line").and_then(|s| s.parse().ok()),
                                target: entry.metadata.get("target").cloned(),
                                metadata,
                            };
                            log_entries.push(log_entry);
                        }
                    }
                }
            }

            Ok(log_entries)
        } else {
            Ok(Vec::new())
        }
    }
}

/// Macro for easier logging with file and line information
#[macro_export]
macro_rules! ally_log {
    ($logger:expr, $level:expr, $msg:expr) => {
        $logger
            .log(
                $level,
                $msg.to_string(),
                Some(module_path!().to_string()),
                Some(file!().to_string()),
                Some(line!()),
                Some(module_path!().to_string()),
                None,
            )
            .await
    };
    ($logger:expr, $level:expr, $msg:expr, $metadata:expr) => {
        $logger
            .log(
                $level,
                $msg.to_string(),
                Some(module_path!().to_string()),
                Some(file!().to_string()),
                Some(line!()),
                Some(module_path!().to_string()),
                Some($metadata),
            )
            .await
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_logger_creation() {
        let config = LoggerConfig::new("test_session".to_string());
        let logger = AllyLogger::new(config);
        assert!(logger.is_ok());
    }

    #[tokio::test]
    async fn test_console_logging() {
        let config =
            LoggerConfig::new("test_session".to_string()).with_console_level(LogLevel::Debug);
        let logger = AllyLogger::new(config).unwrap();

        let result = logger.info("Test message".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_file_logging() {
        let temp_dir = tempdir().unwrap();
        let log_file = temp_dir.path().join("test.log");

        let config = LoggerConfig::new("test_session".to_string())
            .with_file_path(Some(log_file.clone()))
            .with_structured(true);
        let logger = AllyLogger::new(config).unwrap();

        logger.info("Test file message".to_string()).await.unwrap();

        let content = fs::read_to_string(&log_file).unwrap();
        assert!(content.contains("Test file message"));
        assert!(content.contains("test_session"));
    }

    #[test]
    fn test_log_level_parsing() {
        assert_eq!(LogLevel::from_str("error"), LogLevel::Error);
        assert_eq!(LogLevel::from_str("ERROR"), LogLevel::Error);
        assert_eq!(LogLevel::from_str("warn"), LogLevel::Warn);
        assert_eq!(LogLevel::from_str("warning"), LogLevel::Warn);
        assert_eq!(LogLevel::from_str("info"), LogLevel::Info);
        assert_eq!(LogLevel::from_str("debug"), LogLevel::Debug);
        assert_eq!(LogLevel::from_str("trace"), LogLevel::Trace);
        assert_eq!(LogLevel::from_str("unknown"), LogLevel::Info);
    }

    #[test]
    fn test_log_entry_formatting() {
        let entry = LogEntry::new(
            LogLevel::Info,
            "Test message".to_string(),
            "test_session".to_string(),
            Some("test_module".to_string()),
            Some("test.rs".to_string()),
            Some(42),
            Some("test_target".to_string()),
        );

        let console_format = entry.format_console();
        assert!(console_format.contains("INFO"));
        assert!(console_format.contains("Test message"));
        assert!(console_format.contains("test.rs:42"));

        let json_format = entry.format_json().unwrap();
        assert!(json_format.contains("\"level\":\"INFO\""));
        assert!(json_format.contains("\"message\":\"Test message\""));
        assert!(json_format.contains("\"session_id\":\"test_session\""));

        let vector_format = entry.format_vector_store();
        assert!(vector_format.contains("Level: INFO"));
        assert!(vector_format.contains("Message: Test message"));
        assert!(vector_format.contains("Session: test_session"));
    }
}
