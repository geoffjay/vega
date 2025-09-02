use anyhow::Result;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::fs;
use std::path::PathBuf;

use super::ToolError;
use crate::logging::{LogEntry, LogLevel, Logger};

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadLogsArgs {
    /// Session ID to read logs for
    pub session_id: String,
    /// Maximum number of log entries to return (default: 50)
    pub limit: Option<usize>,
    /// Log level filter (error, warn, info, debug, trace)
    pub level_filter: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ReadLogsTool {
    #[serde(skip)]
    logger: Option<std::sync::Arc<Logger>>,
}

impl ReadLogsTool {
    pub fn new() -> Self {
        Self { logger: None }
    }

    pub fn with_logger(mut self, logger: std::sync::Arc<Logger>) -> Self {
        self.logger = Some(logger);
        self
    }

    fn get_log_config() -> (String, Option<PathBuf>) {
        let log_output = env::var("ALLY_LOG_OUTPUT").unwrap_or_else(|_| "console".to_string());
        let log_file = env::var("ALLY_LOG_FILE").ok().map(PathBuf::from);
        (log_output, log_file)
    }

    async fn read_logs_from_file(
        &self,
        session_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<LogEntry>, ToolError> {
        let (_, log_file_path) = Self::get_log_config();

        if let Some(log_file_path) = log_file_path {
            let content = fs::read_to_string(&log_file_path).map_err(|e| ToolError::Io(e))?;

            let mut log_entries = Vec::new();

            // Parse log file content - assuming each line is a log entry
            for line in content.lines().rev() {
                // Reverse to get most recent first
                if line.contains(session_id) {
                    // Try to parse as JSON first (structured logging)
                    if let Ok(entry) = serde_json::from_str::<LogEntry>(line) {
                        log_entries.push(entry);
                    } else {
                        // Parse console format: "YYYY-MM-DD HH:MM:SS.sss UTC [LEVEL] message"
                        if let Some(log_entry) = self.parse_console_log_line(line, session_id) {
                            log_entries.push(log_entry);
                        }
                    }

                    if let Some(limit) = limit {
                        if log_entries.len() >= limit {
                            break;
                        }
                    }
                }
            }

            Ok(log_entries)
        } else {
            Err(ToolError::InvalidInput(
                "No log file path configured".to_string(),
            ))
        }
    }

    async fn read_logs_from_vector_store(
        &self,
        session_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<LogEntry>, ToolError> {
        if let Some(ref logger) = self.logger {
            logger
                .get_session_logs(session_id, limit)
                .await
                .map_err(|e| {
                    ToolError::InvalidInput(format!("Failed to read logs from vector store: {}", e))
                })
        } else {
            Err(ToolError::InvalidInput(
                "Logger not configured for vector store access".to_string(),
            ))
        }
    }

    fn parse_console_log_line(&self, line: &str, session_id: &str) -> Option<LogEntry> {
        // Parse format: "2025-09-01 21:19:32.454 UTC [INFO] Context database: "vega_context.db""
        let parts: Vec<&str> = line.splitn(4, ' ').collect();
        if parts.len() < 4 {
            return None;
        }

        // Extract timestamp
        let timestamp_str = format!("{} {}", parts[0], parts[1]);
        let timestamp = chrono::DateTime::parse_from_str(&timestamp_str, "%Y-%m-%d %H:%M:%S%.3f")
            .ok()?
            .with_timezone(&chrono::Utc);

        // Extract level (between brackets)
        let level_part = parts[3];
        let level_start = level_part.find('[')?;
        let level_end = level_part.find(']')?;
        let level = &level_part[level_start + 1..level_end];

        // Extract message (everything after the level)
        let message_start = level_end + 2; // Skip "] "
        let message = if message_start < level_part.len() {
            &level_part[message_start..]
        } else {
            ""
        };

        Some(LogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp,
            level: level.to_string(),
            message: message.to_string(),
            session_id: session_id.to_string(),
            module: None,
            file: None,
            line: None,
            target: None,
            metadata: std::collections::HashMap::new(),
        })
    }

    fn filter_by_level(
        &self,
        entries: Vec<LogEntry>,
        level_filter: Option<String>,
    ) -> Vec<LogEntry> {
        if let Some(filter_level) = level_filter {
            let filter_level = LogLevel::from_str(&filter_level);
            entries
                .into_iter()
                .filter(|entry| {
                    let entry_level = LogLevel::from_str(&entry.level);
                    entry_level <= filter_level
                })
                .collect()
        } else {
            entries
        }
    }
}

impl Default for ReadLogsTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for ReadLogsTool {
    const NAME: &'static str = "read_logs";

    type Error = ToolError;
    type Args = ReadLogsArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Read log messages for a specific session. Can read from file logs or vector store depending on configuration.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "The session ID to read logs for"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of log entries to return (default: 50)",
                        "minimum": 1,
                        "maximum": 1000
                    },
                    "level_filter": {
                        "type": "string",
                        "description": "Filter logs by level (error, warn, info, debug, trace). Shows specified level and higher priority levels.",
                        "enum": ["error", "warn", "info", "debug", "trace"]
                    }
                },
                "required": ["session_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let limit = args.limit.unwrap_or(50);

        // Determine where to read logs from based on configuration
        let (log_output_type, _) = Self::get_log_config();
        let log_outputs: Vec<&str> = log_output_type.split(',').collect();

        let mut log_entries = if log_outputs.contains(&"vector") {
            // Read from vector store
            self.read_logs_from_vector_store(&args.session_id, Some(limit))
                .await?
        } else if log_outputs.contains(&"file") {
            // Read from file
            self.read_logs_from_file(&args.session_id, Some(limit))
                .await?
        } else {
            return Ok("No log storage configured. Logs are only available when file or vector output is enabled.".to_string());
        };

        // Apply level filter if specified
        log_entries = self.filter_by_level(log_entries, args.level_filter);

        // Sort by timestamp (most recent first)
        log_entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Limit results
        log_entries.truncate(limit);

        if log_entries.is_empty() {
            return Ok(format!(
                "No log entries found for session ID: {}",
                args.session_id
            ));
        }

        // Format output
        let mut output = format!(
            "Found {} log entries for session {}:\n\n",
            log_entries.len(),
            args.session_id
        );

        for entry in log_entries {
            output.push_str(&format!(
                "{} [{}] {}\n",
                entry.timestamp.format("%Y-%m-%d %H:%M:%S%.3f UTC"),
                entry.level,
                entry.message
            ));

            // Add metadata if present
            if !entry.metadata.is_empty() {
                output.push_str("  Metadata: ");
                for (key, value) in &entry.metadata {
                    output.push_str(&format!("{}={} ", key, value));
                }
                output.push('\n');
            }
            output.push('\n');
        }

        Ok(output)
    }
}
