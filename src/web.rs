use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Json},
    routing::get,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
// Web server module - uses custom logger when available

use crate::context::ContextStore;
use crate::logging::Logger;

/// Web server state
#[derive(Clone)]
pub struct WebState {
    pub context_store: Arc<ContextStore>,
    pub logger: Option<Arc<Logger>>,
}

/// Query parameters for context entries
#[derive(Deserialize)]
pub struct ContextQuery {
    pub session_id: Option<String>,
    pub limit: Option<usize>,
}

/// Response for context entries API
#[derive(Serialize)]
pub struct ContextResponse {
    pub entries: Vec<ContextEntryResponse>,
    pub total: usize,
}

/// Serializable context entry for API responses
#[derive(Serialize)]
pub struct ContextEntryResponse {
    pub id: String,
    pub agent_name: String,
    pub session_id: String,
    pub timestamp: String,
    pub content: String,
    pub role: String,
    pub metadata: HashMap<String, String>,
}

/// Response for sessions API
#[derive(Serialize)]
pub struct SessionsResponse {
    pub sessions: Vec<SessionInfoResponse>,
    pub total: usize,
}

/// Response for logs API
#[derive(Serialize)]
pub struct LogsResponse {
    pub logs: Vec<LogEntryResponse>,
    pub total: usize,
}

/// Serializable log entry for API responses
#[derive(Serialize)]
pub struct LogEntryResponse {
    pub id: String,
    pub timestamp: String,
    pub level: String,
    pub message: String,
    pub session_id: String,
    pub module: Option<String>,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub target: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Serializable session info for API responses
#[derive(Serialize)]
pub struct SessionInfoResponse {
    pub session_id: String,
    pub entry_count: usize,
    pub first_entry: String,
    pub last_entry: String,
}

/// Start the web server
pub async fn start_web_server(
    context_store: Arc<ContextStore>,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    start_web_server_with_logger(context_store, None, port).await
}

/// Start the web server with optional logger
pub async fn start_web_server_with_logger(
    context_store: Arc<ContextStore>,
    logger: Option<Arc<Logger>>,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    // Log using custom logger if available, otherwise use println
    if let Some(ref logger) = logger {
        let _ = logger
            .info(format!("Web server starting on http://127.0.0.1:{}", port))
            .await;
    } else {
        println!("Web server starting on http://127.0.0.1:{}", port);
    }

    let state = WebState {
        context_store,
        logger,
    };

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/sessions", get(sessions_handler))
        .route("/api/sessions/:session_id", get(session_handler))
        .route("/api/sessions/:session_id/logs", get(session_logs_handler))
        .route("/api/context", get(context_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;

    axum::serve(listener, app).await?;
    Ok(())
}

/// Serve the main HTML page
async fn index_handler() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

/// Get all sessions
async fn sessions_handler(
    State(state): State<WebState>,
) -> Result<Json<SessionsResponse>, StatusCode> {
    match state.context_store.list_sessions().await {
        Ok(sessions) => {
            let session_responses: Vec<SessionInfoResponse> = sessions
                .into_iter()
                .map(|s| SessionInfoResponse {
                    session_id: s.session_id,
                    entry_count: s.entry_count,
                    first_entry: s.first_entry.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                    last_entry: s.last_entry.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                })
                .collect();

            let total = session_responses.len();
            Ok(Json(SessionsResponse {
                sessions: session_responses,
                total,
            }))
        }
        Err(e) => {
            // Log error if logger is available
            if let Some(ref logger) = state.logger {
                let _ = logger
                    .debug(format!("Error fetching sessions: {}", e))
                    .await;
            }
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get context entries for a specific session
async fn session_handler(
    Path(session_id): Path<String>,
    Query(query): Query<ContextQuery>,
    State(state): State<WebState>,
) -> Result<Json<ContextResponse>, StatusCode> {
    let limit = query.limit.unwrap_or(100);

    match state
        .context_store
        .get_session_history(&session_id, Some(limit))
        .await
    {
        Ok(entries) => {
            let entry_responses: Vec<ContextEntryResponse> = entries
                .into_iter()
                .map(|e| ContextEntryResponse {
                    id: e.id,
                    agent_name: e.agent_name,
                    session_id: e.session_id,
                    timestamp: e.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                    content: e.content,
                    role: e.role,
                    metadata: e.metadata,
                })
                .collect();

            let total = entry_responses.len();
            Ok(Json(ContextResponse {
                entries: entry_responses,
                total,
            }))
        }
        Err(e) => {
            // Log error if logger is available
            if let Some(ref logger) = state.logger {
                let _ = logger
                    .debug(format!("Error fetching session history: {}", e))
                    .await;
            }
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get context entries with optional filtering
async fn context_handler(
    Query(query): Query<ContextQuery>,
    State(state): State<WebState>,
) -> Result<Json<ContextResponse>, StatusCode> {
    let limit = query.limit.unwrap_or(50);

    if let Some(session_id) = query.session_id {
        // Get entries for specific session
        match state
            .context_store
            .get_session_history(&session_id, Some(limit))
            .await
        {
            Ok(entries) => {
                let entry_responses: Vec<ContextEntryResponse> = entries
                    .into_iter()
                    .map(|e| ContextEntryResponse {
                        id: e.id,
                        agent_name: e.agent_name,
                        session_id: e.session_id,
                        timestamp: e.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                        content: e.content,
                        role: e.role,
                        metadata: e.metadata,
                    })
                    .collect();

                let total = entry_responses.len();
                Ok(Json(ContextResponse {
                    entries: entry_responses,
                    total,
                }))
            }
            Err(e) => {
                // Log error if logger is available
                if let Some(ref logger) = state.logger {
                    let _ = logger
                        .debug(format!("Error fetching context entries: {}", e))
                        .await;
                }
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        // For now, return empty response when no session is specified
        // In a full implementation, you might want to return recent entries across all sessions
        Ok(Json(ContextResponse {
            entries: vec![],
            total: 0,
        }))
    }
}

/// Get logs for a specific session
async fn session_logs_handler(
    Path(session_id): Path<String>,
    Query(query): Query<ContextQuery>,
    State(state): State<WebState>,
) -> Result<Json<LogsResponse>, StatusCode> {
    let limit = query.limit.unwrap_or(100);

    if let Some(ref logger) = state.logger {
        match logger.get_session_logs(&session_id, Some(limit)).await {
            Ok(logs) => {
                let log_responses: Vec<LogEntryResponse> = logs
                    .into_iter()
                    .map(|log| LogEntryResponse {
                        id: log.id,
                        timestamp: log
                            .timestamp
                            .format("%Y-%m-%d %H:%M:%S%.3f UTC")
                            .to_string(),
                        level: log.level,
                        message: log.message,
                        session_id: log.session_id,
                        module: log.module,
                        file: log.file,
                        line: log.line,
                        target: log.target,
                        metadata: log.metadata,
                    })
                    .collect();

                let total = log_responses.len();
                Ok(Json(LogsResponse {
                    logs: log_responses,
                    total,
                }))
            }
            Err(e) => {
                // Log error if logger is available
                if let Some(ref logger) = state.logger {
                    let _ = logger
                        .debug(format!("Error fetching session logs: {}", e))
                        .await;
                }
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        // Return empty response if logger is not configured
        Ok(Json(LogsResponse {
            logs: vec![],
            total: 0,
        }))
    }
}
