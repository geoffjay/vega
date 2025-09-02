use anyhow::Result;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::ToolError;

#[derive(Deserialize)]
pub struct WebSearchArgs {
    pub query: String,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
}

fn default_max_results() -> usize {
    5
}

#[derive(Serialize, Debug)]
pub struct WebSearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[derive(Serialize, Debug)]
pub struct WebSearchOutput {
    pub results: Vec<WebSearchResult>,
    pub query: String,
}

pub struct WebSearchTool {
    client: reqwest::Client,
}

impl WebSearchTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Performs a DuckDuckGo instant answer search
    async fn search_duckduckgo(
        &self,
        query: &str,
        max_results: usize,
    ) -> Result<Vec<WebSearchResult>, ToolError> {
        // Using DuckDuckGo's instant answer API as a simple example
        // In a real implementation, you might want to use a proper search API
        let url = format!(
            "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
            urlencoding::encode(query)
        );

        let response = self
            .client
            .get(&url)
            .header("User-Agent", "vega-agent/0.1.0")
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;

        let mut results = Vec::new();

        // Extract abstract if available
        if let Some(abstract_text) = json.get("Abstract").and_then(|v| v.as_str()) {
            if !abstract_text.is_empty() {
                if let Some(abstract_url) = json.get("AbstractURL").and_then(|v| v.as_str()) {
                    results.push(WebSearchResult {
                        title: json
                            .get("AbstractSource")
                            .and_then(|v| v.as_str())
                            .unwrap_or("DuckDuckGo")
                            .to_string(),
                        url: abstract_url.to_string(),
                        snippet: abstract_text.to_string(),
                    });
                }
            }
        }

        // Extract related topics
        if let Some(related_topics) = json.get("RelatedTopics").and_then(|v| v.as_array()) {
            for topic in related_topics
                .iter()
                .take(max_results.saturating_sub(results.len()))
            {
                if let (Some(text), Some(url)) = (
                    topic.get("Text").and_then(|v| v.as_str()),
                    topic.get("FirstURL").and_then(|v| v.as_str()),
                ) {
                    results.push(WebSearchResult {
                        title: "Related Topic".to_string(),
                        url: url.to_string(),
                        snippet: text.to_string(),
                    });
                }
            }
        }

        // If no results from DuckDuckGo, provide a fallback message
        if results.is_empty() {
            results.push(WebSearchResult {
                title: "Search Query".to_string(),
                url: format!("https://duckduckgo.com/?q={}", urlencoding::encode(query)),
                snippet: format!("No instant results found for '{}'. You can search manually at the provided URL.", query),
            });
        }

        Ok(results)
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for WebSearchTool {
    const NAME: &'static str = "web_search";
    type Error = ToolError;
    type Args = WebSearchArgs;
    type Output = WebSearchOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Performs a web search and returns relevant results with titles, URLs, and snippets.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query string"
                    },
                    "max_results": {
                        "type": "number",
                        "description": "Maximum number of results to return (default: 5)",
                        "default": 5
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let results = self
            .search_duckduckgo(&args.query, args.max_results)
            .await?;

        Ok(WebSearchOutput {
            results,
            query: args.query,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_search_tool_creation() {
        let tool = WebSearchTool::new();
        assert_eq!(WebSearchTool::NAME, "web_search");
    }

    #[test]
    fn test_default_max_results() {
        assert_eq!(default_max_results(), 5);
    }

    #[tokio::test]
    async fn test_web_search_definition() {
        let tool = WebSearchTool::new();
        let definition = tool.definition("test prompt".to_string()).await;

        assert_eq!(definition.name, "web_search");
        assert!(!definition.description.is_empty());
    }
}
