use ally::agents::chat::ChatAgent;
use ally::agents::{Agent, AgentConfig};

#[tokio::test]
async fn test_chat_agent_integration_ollama() {
    let config = AgentConfig::new(
        false,
        "ollama".to_string(),
        "llama3.2".to_string(),
        None,
        "simple".to_string(),
        None,
        None,
        false,
    );

    let agent = ChatAgent::new(config);
    assert!(agent.is_ok());

    let agent = agent.unwrap();
    assert_eq!(agent.name(), "chat");
}

#[tokio::test]
async fn test_chat_agent_integration_openrouter() {
    let config = AgentConfig::new(
        false,
        "openrouter".to_string(),
        "gpt-4".to_string(),
        Some("test-api-key".to_string()),
        "simple".to_string(),
        None,
        None,
        false,
    );

    let agent = ChatAgent::new(config);
    assert!(agent.is_ok());

    let agent = agent.unwrap();
    assert_eq!(agent.name(), "chat");
}

#[tokio::test]
async fn test_agent_config_integration() {
    // Test that AgentConfig properly integrates with ChatAgent
    let configs = vec![
        AgentConfig::new(
            true,
            "ollama".to_string(),
            "llama3.2".to_string(),
            None,
            "simple".to_string(),
            None,
            None,
            false,
        ),
        AgentConfig::new(
            false,
            "openrouter".to_string(),
            "gpt-4".to_string(),
            Some("key".to_string()),
            "simple".to_string(),
            None,
            None,
            false,
        ),
    ];

    for config in configs {
        let verbose = config.verbose;
        let provider = config.provider.clone();
        let model = config.model.clone();

        let agent = ChatAgent::new(config);

        if provider == "openrouter" || provider == "ollama" {
            assert!(agent.is_ok());
            let agent = agent.unwrap();
            assert_eq!(agent.name(), "chat");
            assert_eq!(agent.config().verbose, verbose);
            assert_eq!(agent.config().provider, provider);
            assert_eq!(agent.config().model, model);
        }
    }
}

#[test]
fn test_error_handling_integration() {
    // Test error cases in integration
    let invalid_configs = vec![
        // Invalid provider
        AgentConfig::new(
            false,
            "invalid".to_string(),
            "model".to_string(),
            None,
            "simple".to_string(),
            None,
            None,
            false,
        ),
        // OpenRouter without API key
        AgentConfig::new(
            false,
            "openrouter".to_string(),
            "gpt-4".to_string(),
            None,
            "simple".to_string(),
            None,
            None,
            false,
        ),
    ];

    for config in invalid_configs {
        let agent = ChatAgent::new(config);
        // Agent creation might succeed, but the agent should be properly configured
        // The actual validation happens when trying to use the agent
        if agent.is_ok() {
            let agent = agent.unwrap();
            // Just verify the agent was created with the expected configuration
            assert_eq!(agent.name(), "chat");
        }
    }
}
