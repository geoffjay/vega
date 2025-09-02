//! Simple test for ACP integration
//!
//! This example demonstrates how to test the ACP integration by sending
//! a simple message to the agent and receiving a response.

use agent_client_protocol as acp;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Ally ACP integration...");

    // Start Ally in ACP mode
    let mut child = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "--acp",
            "--provider",
            "ollama",
            "--model",
            "llama3.2",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdin = child.stdin.as_mut().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut reader = BufReader::new(stdout);

    // Send initialize request
    let init_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocol_version": "v1",
            "client_capabilities": {
                "file_operations": true
            }
        }
    });

    let request_str = format!("{}\n", init_request);
    stdin.write_all(request_str.as_bytes()).await?;
    stdin.flush().await?;

    // Read response
    let mut response = String::new();
    reader.read_line(&mut response).await?;

    println!("Initialize response: {}", response.trim());

    // Send new session request
    let session_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "new_session",
        "params": {
            "mcp_servers": [],
            "cwd": std::env::current_dir()?
        }
    });

    let request_str = format!("{}\n", session_request);
    stdin.write_all(request_str.as_bytes()).await?;
    stdin.flush().await?;

    // Read response
    let mut response = String::new();
    reader.read_line(&mut response).await?;

    println!("New session response: {}", response.trim());

    // Parse session ID from response
    let session_response: serde_json::Value = serde_json::from_str(&response)?;
    let session_id = session_response["result"]["session_id"]
        .as_str()
        .expect("No session ID in response");

    // Send a prompt
    let prompt_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "prompt",
        "params": {
            "session_id": session_id,
            "prompt": [
                {
                    "type": "text",
                    "text": "Hello! Can you tell me about yourself?"
                }
            ]
        }
    });

    let request_str = format!("{}\n", prompt_request);
    stdin.write_all(request_str.as_bytes()).await?;
    stdin.flush().await?;

    // Read prompt response
    let mut response = String::new();
    reader.read_line(&mut response).await?;

    println!("Prompt response: {}", response.trim());

    // Read any session notifications (agent messages)
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).await? > 0 {
                if line.trim().contains("session_notification") {
                    println!("Agent message: {}", line.trim());
                }
            } else {
                break;
            }
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    })
    .await
    .ok();

    // Clean up
    child.kill().await?;

    println!("ACP test completed!");
    Ok(())
}
