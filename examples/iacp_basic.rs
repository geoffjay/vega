//! Basic example demonstrating IaCP integration with the main Vega crate

use serde_json::json;
use vega::iacp::{AgentInfo, AgentRegistry, IacpMessage, MessageMetadata, Priority, Recipient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🤖 Vega IaCP Integration Test");
    println!("=============================");

    // Test 1: Create an agent registry
    let registry = AgentRegistry::new();
    println!("✅ Created agent registry");

    // Test 2: Create sample agent info
    let agent = AgentInfo {
        agent_id: "test-agent-001".to_string(),
        agent_name: "Test Agent".to_string(),
        capabilities: vec!["chat".to_string(), "analysis".to_string()],
    };

    // Test 3: Register the agent
    registry.register_agent(agent.clone()).await?;
    println!(
        "✅ Registered agent: {} ({})",
        agent.agent_name, agent.agent_id
    );

    // Test 4: Find agents by capability
    let chat_agents = registry.find_agents_by_capability("chat").await;
    println!(
        "✅ Found {} agents with 'chat' capability",
        chat_agents.len()
    );

    // Test 5: Create a sample IaCP message
    let recipient = Recipient {
        agent_id: Some("target-agent".to_string()),
        broadcast: false,
    };

    let payload = json!({
        "task_type": "greeting",
        "message": "Hello from Vega IaCP!"
    });

    let message = IacpMessage::new(
        agent.clone(),
        recipient,
        "task_request".to_string(),
        payload,
    );

    println!("✅ Created IaCP message with ID: {}", message.message_id);

    // Test 6: Convert message to bytes and back
    let bytes = message.to_bytes()?;
    let parsed_message = IacpMessage::from_bytes(&bytes)?;
    println!("✅ Successfully serialized and deserialized message");
    println!("   Message type: {}", parsed_message.message_type);
    println!("   Sender: {}", parsed_message.sender.agent_name);

    // Test 7: Test message metadata
    let mut message_with_metadata = message;
    message_with_metadata.metadata = MessageMetadata {
        priority: Priority::High,
        expires_at: None,
        requires_response: true,
        response_timeout: Some(30),
    };

    println!(
        "✅ Set message priority to: {:?}",
        message_with_metadata.metadata.priority
    );

    println!("\n🎉 All IaCP integration tests passed!");
    println!("The vega-iacp crate is successfully integrated with the main Vega project.");

    Ok(())
}
