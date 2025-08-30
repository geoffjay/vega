use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{debug, info};
use uuid::Uuid;

/// Represents a single context entry in the vector store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    pub id: String,
    pub agent_name: String,
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub content: String,
    pub role: String, // "user" or "assistant"
    pub metadata: HashMap<String, String>,
}

impl ContextEntry {
    pub fn new(agent_name: String, session_id: String, content: String, role: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            agent_name,
            session_id,
            timestamp: Utc::now(),
            content,
            role,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Context store for managing conversation history and cross-agent context
/// Uses SQLite for single-file storage with simple vector similarity via cosine distance
pub struct ContextStore {
    connection: Arc<Mutex<Connection>>,
    embedding_dim: usize,
}

impl ContextStore {
    /// Create a new context store with the specified database path
    pub async fn new<P: AsRef<Path>>(db_path: P, embedding_dim: usize) -> Result<Self> {
        let connection =
            Connection::open(db_path.as_ref()).context("Failed to open SQLite database")?;

        let store = Self {
            connection: Arc::new(Mutex::new(connection)),
            embedding_dim,
        };

        store.initialize_tables().await?;
        Ok(store)
    }

    /// Initialize the context tables with the proper schema
    async fn initialize_tables(&self) -> Result<()> {
        let conn = self.connection.lock().unwrap();

        // Create context entries table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS context_entries (
                id TEXT PRIMARY KEY,
                agent_name TEXT NOT NULL,
                session_id TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                content TEXT NOT NULL,
                role TEXT NOT NULL,
                metadata TEXT NOT NULL
            )",
            [],
        )?;

        // Create embeddings table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS embeddings (
                entry_id TEXT PRIMARY KEY,
                embedding BLOB NOT NULL,
                FOREIGN KEY(entry_id) REFERENCES context_entries(id)
            )",
            [],
        )?;

        // Create indexes for better performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_session_id ON context_entries(session_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_timestamp ON context_entries(timestamp)",
            [],
        )?;

        info!("Context store tables initialized");
        Ok(())
    }

    /// Store a context entry with its embedding
    pub async fn store_context(&self, entry: ContextEntry, embedding: Vec<f32>) -> Result<()> {
        if embedding.len() != self.embedding_dim {
            return Err(anyhow::anyhow!(
                "Embedding dimension mismatch: expected {}, got {}",
                self.embedding_dim,
                embedding.len()
            ));
        }

        let conn = self.connection.lock().unwrap();

        // Store context entry
        conn.execute(
            "INSERT INTO context_entries (id, agent_name, session_id, timestamp, content, role, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                entry.id,
                entry.agent_name,
                entry.session_id,
                entry.timestamp.timestamp(),
                entry.content,
                entry.role,
                serde_json::to_string(&entry.metadata)?
            ],
        )?;

        // Store embedding as binary data
        let embedding_bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();

        conn.execute(
            "INSERT INTO embeddings (entry_id, embedding) VALUES (?1, ?2)",
            params![entry.id, embedding_bytes],
        )?;

        debug!("Stored context entry: {}", entry.id);
        Ok(())
    }

    /// Retrieve relevant context entries using simple cosine similarity
    pub async fn get_relevant_context(
        &self,
        query_embedding: Vec<f32>,
        session_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ContextEntry>> {
        if query_embedding.len() != self.embedding_dim {
            return Err(anyhow::anyhow!(
                "Query embedding dimension mismatch: expected {}, got {}",
                self.embedding_dim,
                query_embedding.len()
            ));
        }

        let conn = self.connection.lock().unwrap();

        // Get all embeddings and calculate similarity
        let mut stmt = conn.prepare(
            "SELECT ce.id, ce.agent_name, ce.session_id, ce.timestamp, ce.content, ce.role, ce.metadata, e.embedding
             FROM context_entries ce
             JOIN embeddings e ON ce.id = e.entry_id
             ORDER BY ce.timestamp DESC"
        )?;

        let mut entries_with_scores = Vec::new();

        let rows = stmt.query_map([], |row| {
            let metadata_json: String = row.get(6)?;
            let metadata: HashMap<String, String> =
                serde_json::from_str(&metadata_json).unwrap_or_default();

            let timestamp =
                DateTime::from_timestamp(row.get::<_, i64>(3)?, 0).unwrap_or_else(Utc::now);

            let embedding_bytes: Vec<u8> = row.get(7)?;
            let embedding: Vec<f32> = embedding_bytes
                .chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect();

            let entry = ContextEntry {
                id: row.get(0)?,
                agent_name: row.get(1)?,
                session_id: row.get(2)?,
                timestamp,
                content: row.get(4)?,
                role: row.get(5)?,
                metadata,
            };

            Ok((entry, embedding))
        })?;

        for row_result in rows {
            let (entry, embedding) = row_result?;

            // Filter by session if specified
            if let Some(session_id) = session_id {
                if entry.session_id != session_id {
                    continue;
                }
            }

            // Calculate cosine similarity
            let similarity = self.cosine_similarity(&query_embedding, &embedding);
            entries_with_scores.push((entry, similarity));
        }

        // Sort by similarity (descending) and take top N
        entries_with_scores
            .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        entries_with_scores.truncate(limit);

        let entries: Vec<ContextEntry> = entries_with_scores
            .into_iter()
            .map(|(entry, _)| entry)
            .collect();

        debug!("Retrieved {} relevant context entries", entries.len());
        Ok(entries)
    }

    /// Get conversation history for a specific session
    pub async fn get_session_history(
        &self,
        session_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<ContextEntry>> {
        let conn = self.connection.lock().unwrap();

        let (query, params): (String, Vec<Box<dyn rusqlite::ToSql>>) = match limit {
            Some(limit) => (
                "SELECT id, agent_name, session_id, timestamp, content, role, metadata 
                 FROM context_entries 
                 WHERE session_id = ?1 
                 ORDER BY timestamp ASC 
                 LIMIT ?2"
                    .to_string(),
                vec![Box::new(session_id.to_string()), Box::new(limit as i64)],
            ),
            None => (
                "SELECT id, agent_name, session_id, timestamp, content, role, metadata 
                 FROM context_entries 
                 WHERE session_id = ?1 
                 ORDER BY timestamp ASC"
                    .to_string(),
                vec![Box::new(session_id.to_string())],
            ),
        };

        let mut stmt = conn.prepare(&query)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let entries = stmt
            .query_map(&param_refs[..], |row| {
                let metadata_json: String = row.get(6)?;
                let metadata: HashMap<String, String> =
                    serde_json::from_str(&metadata_json).unwrap_or_default();

                let timestamp =
                    DateTime::from_timestamp(row.get::<_, i64>(3)?, 0).unwrap_or_else(Utc::now);

                Ok(ContextEntry {
                    id: row.get(0)?,
                    agent_name: row.get(1)?,
                    session_id: row.get(2)?,
                    timestamp,
                    content: row.get(4)?,
                    role: row.get(5)?,
                    metadata,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        debug!("Retrieved {} session history entries", entries.len());
        Ok(entries)
    }

    /// Clear all context entries for a specific session
    pub async fn clear_session(&self, session_id: &str) -> Result<()> {
        let conn = self.connection.lock().unwrap();

        // Delete embeddings first (foreign key constraint)
        conn.execute(
            "DELETE FROM embeddings WHERE entry_id IN (
                SELECT id FROM context_entries WHERE session_id = ?1
            )",
            params![session_id],
        )?;

        // Delete context entries
        conn.execute(
            "DELETE FROM context_entries WHERE session_id = ?1",
            params![session_id],
        )?;

        info!("Cleared context for session: {}", session_id);
        Ok(())
    }

    /// Get statistics about the context store
    pub async fn get_stats(&self) -> Result<ContextStats> {
        let conn = self.connection.lock().unwrap();

        let mut stmt = conn.prepare("SELECT COUNT(*) FROM context_entries")?;
        let total_entries: i64 = stmt.query_row([], |row| row.get(0))?;

        Ok(ContextStats {
            total_entries: total_entries as usize,
            embedding_dimension: self.embedding_dim,
        })
    }

    /// List all session IDs that have context entries
    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let conn = self.connection.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT session_id, COUNT(*) as entry_count, MIN(timestamp) as first_entry, MAX(timestamp) as last_entry
             FROM context_entries 
             GROUP BY session_id 
             ORDER BY last_entry DESC"
        )?;

        let sessions = stmt
            .query_map([], |row| {
                let first_timestamp =
                    DateTime::from_timestamp(row.get::<_, i64>(2)?, 0).unwrap_or_else(Utc::now);
                let last_timestamp =
                    DateTime::from_timestamp(row.get::<_, i64>(3)?, 0).unwrap_or_else(Utc::now);

                Ok(SessionInfo {
                    session_id: row.get(0)?,
                    entry_count: row.get::<_, i64>(1)? as usize,
                    first_entry: first_timestamp,
                    last_entry: last_timestamp,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        debug!("Retrieved {} sessions", sessions.len());
        Ok(sessions)
    }

    /// Check if a session exists in the database
    pub async fn session_exists(&self, session_id: &str) -> Result<bool> {
        let conn = self.connection.lock().unwrap();

        let mut stmt =
            conn.prepare("SELECT COUNT(*) FROM context_entries WHERE session_id = ?1")?;
        let count: i64 = stmt.query_row(params![session_id], |row| row.get(0))?;

        Ok(count > 0)
    }

    /// Calculate cosine similarity between two embeddings
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }
}

/// Statistics about the context store
#[derive(Debug, Clone)]
pub struct ContextStats {
    pub total_entries: usize,
    pub embedding_dimension: usize,
}

/// Information about a session
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: String,
    pub entry_count: usize,
    pub first_entry: DateTime<Utc>,
    pub last_entry: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_context_store_creation() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let store = ContextStore::new(&db_path, 384).await;
        assert!(store.is_ok());
    }

    #[tokio::test]
    async fn test_store_and_retrieve_context() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let store = ContextStore::new(&db_path, 3).await.unwrap();

        let entry = ContextEntry::new(
            "test_agent".to_string(),
            "session_123".to_string(),
            "Hello, world!".to_string(),
            "user".to_string(),
        );

        let embedding = vec![0.1, 0.2, 0.3];
        let result = store.store_context(entry.clone(), embedding.clone()).await;
        assert!(result.is_ok());

        let query_embedding = vec![0.1, 0.2, 0.3];
        let retrieved = store
            .get_relevant_context(query_embedding, Some("session_123"), 10)
            .await
            .unwrap();

        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].content, "Hello, world!");
        assert_eq!(retrieved[0].agent_name, "test_agent");
    }

    #[tokio::test]
    async fn test_session_history() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let store = ContextStore::new(&db_path, 3).await.unwrap();

        // Store multiple entries
        for i in 0..3 {
            let entry = ContextEntry::new(
                "test_agent".to_string(),
                "session_123".to_string(),
                format!("Message {}", i),
                if i % 2 == 0 { "user" } else { "assistant" }.to_string(),
            );
            let embedding = vec![i as f32, (i + 1) as f32, (i + 2) as f32];
            store.store_context(entry, embedding).await.unwrap();
        }

        let history = store
            .get_session_history("session_123", None)
            .await
            .unwrap();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].content, "Message 0");
        assert_eq!(history[2].content, "Message 2");
    }

    #[tokio::test]
    async fn test_clear_session() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let store = ContextStore::new(&db_path, 3).await.unwrap();

        let entry = ContextEntry::new(
            "test_agent".to_string(),
            "session_123".to_string(),
            "Hello, world!".to_string(),
            "user".to_string(),
        );

        let embedding = vec![0.1, 0.2, 0.3];
        store.store_context(entry, embedding).await.unwrap();

        let history_before = store
            .get_session_history("session_123", None)
            .await
            .unwrap();
        assert_eq!(history_before.len(), 1);

        store.clear_session("session_123").await.unwrap();

        let history_after = store
            .get_session_history("session_123", None)
            .await
            .unwrap();
        assert_eq!(history_after.len(), 0);
    }

    #[tokio::test]
    async fn test_cosine_similarity() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let store = ContextStore::new(&db_path, 3).await.unwrap();

        // Test identical vectors
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let similarity = store.cosine_similarity(&a, &b);
        assert!((similarity - 1.0).abs() < 1e-6);

        // Test orthogonal vectors
        let c = vec![1.0, 0.0, 0.0];
        let d = vec![0.0, 1.0, 0.0];
        let similarity = store.cosine_similarity(&c, &d);
        assert!((similarity - 0.0).abs() < 1e-6);
    }
}
