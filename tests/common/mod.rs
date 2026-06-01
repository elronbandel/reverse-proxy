use reqwest::Client;
use serde_json::{json, Value};

#[derive(Clone)]
pub struct TestProxy {
    pub openai_url: String,
    pub mcp_url: String,
    client: Client,
}

impl TestProxy {
    pub async fn start() -> Self {
        todo!("start proxy server on random ports")
    }

    pub async fn openai_chat(&self, body: Value) -> Value {
        self.client
            .post(format!("{}/v1/chat/completions", self.openai_url))
            .json(&body)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap()
    }

    pub async fn mcp_list_tools(&self) -> Vec<String> {
        let result = self.mcp_rpc("tools/list", json!({})).await;
        result["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect()
    }

    pub async fn mcp_call_tool(&self, name: &str, arguments: Value) -> Value {
        let result = self
            .mcp_rpc("tools/call", json!({ "name": name, "arguments": arguments }))
            .await;
        result["content"][0]["text"].clone()
    }

    async fn mcp_rpc(&self, method: &str, params: Value) -> Value {
        let body: Value = self
            .client
            .post(format!("{}/mcp", self.mcp_url))
            .json(&json!({
                "jsonrpc": "2.0",
                "id":      1,
                "method":  method,
                "params":  params
            }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        body["result"].clone()
    }
}

pub fn weather_tool() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "get_weather",
            "description": "Get current weather for a location",
            "parameters": {
                "type": "object",
                "properties": {
                    "location": { "type": "string" }
                },
                "required": ["location"]
            }
        }
    })
}
