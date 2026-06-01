use reqwest::Client;
use serde_json::{json, Value};
use tokio::time::{sleep, Duration};

// ── TestProxy ──────────────────────────────────────────────────────────────

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

    pub async fn mcp_read_message(&self) -> Value {
        let result = self.mcp_rpc("tools/call", json!({"name":"read_message","arguments":{}})).await;
        let text = result["content"][0]["text"].as_str().unwrap();
        serde_json::from_str(text).unwrap()
    }

    pub async fn mcp_write_message(&self, content: &str) {
        self.mcp_rpc("tools/call", json!({"name":"write_message","arguments":{"content":content}})).await;
    }

    pub async fn mcp_call_tool(&self, name: &str, arguments: Value) -> String {
        let result = self.mcp_rpc("tools/call", json!({"name":name,"arguments":arguments})).await;
        result["content"][0]["text"].as_str().unwrap().to_string()
    }

    pub async fn mcp_subscribe_notifications(&self) -> tokio::sync::mpsc::Receiver<String> {
        todo!("subscribe to MCP SSE notification stream")
    }

    async fn mcp_rpc(&self, method: &str, params: Value) -> Value {
        let body: Value = self
            .client
            .post(format!("{}/mcp", self.mcp_url))
            .json(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        body["result"].clone()
    }
}

// ── Assertion helpers ──────────────────────────────────────────────────────

/// Send `input` to the OpenAI endpoint. Agent reads it, replies with `target`.
/// Asserts the caller receives `target` with finish_reason: stop.
pub async fn assert_text_reply(proxy: &TestProxy, input: Value, target: &str) {
    let p = proxy.clone();
    let openai_task = tokio::spawn(async move { p.openai_chat(input).await });

    proxy.mcp_read_message().await;
    proxy.mcp_write_message(target).await;

    let response = openai_task.await.unwrap();
    assert_eq!(response["choices"][0]["message"]["role"], "assistant");
    assert_eq!(response["choices"][0]["message"]["content"], target);
    assert_eq!(response["choices"][0]["finish_reason"], "stop");
}

/// Send `input`, yield for it to become active, assert MCP tools list equals `target`.
pub async fn assert_mcp_tool_list(proxy: &TestProxy, input: Value, target: &[&str]) {
    let p = proxy.clone();
    tokio::spawn(async move { p.openai_chat(input).await });
    sleep(Duration::from_millis(10)).await;

    assert_eq!(proxy.mcp_list_tools().await, target);

    proxy.mcp_write_message("done").await;
}

/// Full tool round-trip: agent calls `tool_name` → codebase sends `tool_return` →
/// agent writes `target`. Asserts tool_calls response shape and final text response.
pub async fn assert_tool_round_trip(
    proxy: &TestProxy,
    input: Value,
    tool_name: &str,
    tool_return: &str,
    target: &str,
) {
    let messages = input["messages"].clone();
    let p = proxy.clone();
    let first_task = tokio::spawn(async move { p.openai_chat(input).await });

    proxy.mcp_read_message().await;

    let p = proxy.clone();
    let name = tool_name.to_string();
    let tool_task = tokio::spawn(async move { p.mcp_call_tool(&name, json!({})).await });

    let tool_call_response = first_task.await.unwrap();
    assert_eq!(tool_call_response["choices"][0]["finish_reason"], "tool_calls");
    assert_eq!(tool_call_response["choices"][0]["message"]["content"], Value::Null);
    let tc = &tool_call_response["choices"][0]["message"]["tool_calls"][0];
    assert_eq!(tc["function"]["name"], tool_name);

    let mut msgs = messages.as_array().unwrap().clone();
    msgs.push(tool_call_response["choices"][0]["message"].clone());
    msgs.push(json!({"role":"tool","content":tool_return,"tool_call_id":tc["id"]}));

    let p = proxy.clone();
    let second_task = tokio::spawn(async move { p.openai_chat(json!({"messages":msgs})).await });

    assert_eq!(tool_task.await.unwrap(), tool_return);

    proxy.mcp_write_message(target).await;

    let final_response = second_task.await.unwrap();
    assert_eq!(final_response["choices"][0]["message"]["content"], target);
    assert_eq!(final_response["choices"][0]["finish_reason"], "stop");
}

/// Send all `inputs` concurrently, assert agent sees them in arrival order.
pub async fn assert_fifo_order(proxy: &TestProxy, inputs: Vec<Value>) {
    let expected: Vec<String> = inputs
        .iter()
        .map(|i| i["messages"][0]["content"].as_str().unwrap().to_string())
        .collect();

    for input in inputs {
        let p = proxy.clone();
        tokio::spawn(async move { p.openai_chat(input).await });
    }

    for content in &expected {
        let read = proxy.mcp_read_message().await;
        assert_eq!(read["messages"][0]["content"], *content);
        proxy.mcp_write_message("reply").await;
    }
}

/// After a tool round-trip, assert read_message returns only `expected_delta`
/// and conversation_id is unchanged.
pub async fn assert_delta_on_continuation(
    proxy: &TestProxy,
    input: Value,
    tool_name: &str,
    tool_return: &str,
    expected_delta: Value,
) {
    let messages = input["messages"].clone();
    let p = proxy.clone();
    let first_task = tokio::spawn(async move { p.openai_chat(input).await });

    let first_read = proxy.mcp_read_message().await;
    let conversation_id = first_read["conversation_id"].clone();

    let p = proxy.clone();
    let name = tool_name.to_string();
    let tool_task = tokio::spawn(async move { p.mcp_call_tool(&name, json!({})).await });

    let tool_call_response = first_task.await.unwrap();
    let tc = &tool_call_response["choices"][0]["message"]["tool_calls"][0];

    let mut msgs = messages.as_array().unwrap().clone();
    msgs.push(tool_call_response["choices"][0]["message"].clone());
    msgs.push(json!({"role":"tool","content":tool_return,"tool_call_id":tc["id"]}));

    let p = proxy.clone();
    tokio::spawn(async move { p.openai_chat(json!({"messages":msgs})).await });
    tool_task.await.unwrap();

    let delta = proxy.mcp_read_message().await;
    assert_eq!(delta["conversation_id"], conversation_id, "conversation_id changed on continuation");
    assert_eq!(delta["messages"], expected_delta);

    proxy.mcp_write_message("done").await;
}

/// Two independent requests get different conversation_ids.
pub async fn assert_different_conversation_ids(
    proxy: &TestProxy,
    first_input: Value,
    second_input: Value,
) {
    let p = proxy.clone();
    tokio::spawn(async move { p.openai_chat(first_input).await });
    let first_id = proxy.mcp_read_message().await["conversation_id"].clone();
    proxy.mcp_write_message("reply").await;

    let p = proxy.clone();
    tokio::spawn(async move { p.openai_chat(second_input).await });
    let second_id = proxy.mcp_read_message().await["conversation_id"].clone();
    proxy.mcp_write_message("reply").await;

    assert_ne!(first_id, second_id);
}

/// After turn 1 completes, turn 2 exposes only `expected_tools` — no leakage.
pub async fn assert_no_state_leakage(
    proxy: &TestProxy,
    first_input: Value,
    second_input: Value,
    expected_tools: &[&str],
) {
    let p = proxy.clone();
    tokio::spawn(async move { p.openai_chat(first_input).await });
    proxy.mcp_read_message().await;
    proxy.mcp_write_message("done").await;
    sleep(Duration::from_millis(10)).await;

    let p = proxy.clone();
    tokio::spawn(async move { p.openai_chat(second_input).await });
    sleep(Duration::from_millis(10)).await;

    assert_eq!(proxy.mcp_list_tools().await, expected_tools);
    proxy.mcp_write_message("done").await;
}

/// `count` requests are all accepted at the HTTP layer before any turn completes.
pub async fn assert_concurrent_ingestion(proxy: &TestProxy, count: usize) {
    let handles: Vec<_> = (0..count)
        .map(|i| {
            let p = proxy.clone();
            tokio::spawn(async move {
                p.openai_chat(json!({"messages":[{"role":"user","content":format!("message {i}")}]})).await
            })
        })
        .collect();

    for _ in 0..count {
        proxy.mcp_read_message().await;
        proxy.mcp_write_message("reply").await;
    }

    for h in handles {
        h.await.unwrap();
    }
}
