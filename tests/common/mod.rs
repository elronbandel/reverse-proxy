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
        let result = self.mcp_call("tools/list", json!({})).await;
        result["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect()
    }

    pub async fn mcp_get_tool_definition(&self, name: &str) -> Value {
        let result = self.mcp_call("tools/list", json!({})).await;
        result["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == name)
            .expect("tool not found")
            .clone()
    }

    pub async fn mcp_read_message(&self) -> Value {
        let result = self.mcp_call("tools/call", json!({"name":"read_message","arguments":{}})).await;
        let text = result["content"][0]["text"].as_str().unwrap();
        serde_json::from_str(text).unwrap()
    }

    pub async fn mcp_write_message(&self, content: &str) {
        self.mcp_call("tools/call", json!({"name":"write_message","arguments":{"content":content}})).await;
    }

    pub async fn mcp_call_tool(&self, name: &str, arguments: Value) -> String {
        let result = self.mcp_call("tools/call", json!({"name":name,"arguments":arguments})).await;
        result["content"][0]["text"].as_str().unwrap().to_string()
    }

    pub async fn mcp_subscribe_notifications(&self) -> tokio::sync::mpsc::Receiver<String> {
        todo!("subscribe to MCP SSE notification stream")
    }

    async fn mcp_call(&self, method: &str, params: Value) -> Value {
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

// ── Shared utility ─────────────────────────────────────────────────────────

/// Append an assistant tool_call message and the corresponding tool result to `history`.
fn with_tool_result(history: &[Value], tool_call_response: &Value, tool_return: &str) -> Vec<Value> {
    let tc = &tool_call_response["choices"][0]["message"]["tool_calls"][0];
    let mut next = history.to_vec();
    next.push(tool_call_response["choices"][0]["message"].clone());
    next.push(json!({"role":"tool","content":tool_return,"tool_call_id":tc["id"]}));
    next
}

// ── Assertion helpers ──────────────────────────────────────────────────────

/// Agent reads `input`, replies with `target`. Asserts finish_reason: stop.
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

/// Sends `input`, asserts MCP tools list equals `target` exactly.
pub async fn assert_mcp_tool_list(proxy: &TestProxy, input: Value, target: &[&str]) {
    let p = proxy.clone();
    tokio::spawn(async move { p.openai_chat(input).await });
    sleep(Duration::from_millis(10)).await;

    assert_eq!(proxy.mcp_list_tools().await, target);

    proxy.mcp_write_message("done").await;
}

/// Sends `input`, asserts the named tool's full definition in `tools/list` equals `target`.
pub async fn assert_dynamic_tool_schema(proxy: &TestProxy, input: Value, tool_name: &str, target: Value) {
    let p = proxy.clone();
    tokio::spawn(async move { p.openai_chat(input).await });
    sleep(Duration::from_millis(10)).await;

    assert_eq!(proxy.mcp_get_tool_definition(tool_name).await, target);

    proxy.mcp_write_message("done").await;
}

/// One tool round-trip: agent calls `tool_name`, codebase returns `tool_return`,
/// agent writes `target`. Asserts tool_calls shape and final text response.
pub async fn assert_tool_round_trip(
    proxy: &TestProxy,
    input: Value,
    tool_name: &str,
    tool_return: &str,
    target: &str,
) {
    let base = input["messages"].as_array().unwrap().clone();
    let p = proxy.clone();
    let first_task = tokio::spawn(async move { p.openai_chat(input).await });

    proxy.mcp_read_message().await;

    let p = proxy.clone();
    let name = tool_name.to_string();
    let tool_task = tokio::spawn(async move { p.mcp_call_tool(&name, json!({})).await });

    let tool_call_response = first_task.await.unwrap();
    assert_eq!(tool_call_response["choices"][0]["finish_reason"], "tool_calls");
    assert_eq!(tool_call_response["choices"][0]["message"]["content"], Value::Null);
    assert_eq!(tool_call_response["choices"][0]["message"]["tool_calls"][0]["function"]["name"], tool_name);

    let continuation = with_tool_result(&base, &tool_call_response, tool_return);
    let p = proxy.clone();
    let final_task = tokio::spawn(async move { p.openai_chat(json!({"messages": continuation})).await });

    assert_eq!(tool_task.await.unwrap(), tool_return);
    proxy.mcp_write_message(target).await;

    let final_response = final_task.await.unwrap();
    assert_eq!(final_response["choices"][0]["message"]["content"], target);
    assert_eq!(final_response["choices"][0]["finish_reason"], "stop");
}

/// Two sequential tool calls in one turn, then agent writes `target`.
pub async fn assert_two_tool_calls_in_one_turn(
    proxy: &TestProxy,
    input: Value,
    first_tool: &str,
    first_return: &str,
    second_tool: &str,
    second_return: &str,
    target: &str,
) {
    let base = input["messages"].as_array().unwrap().clone();
    let p = proxy.clone();
    let first_task = tokio::spawn(async move { p.openai_chat(input).await });

    proxy.mcp_read_message().await;

    // agent calls first tool
    let p = proxy.clone();
    let name = first_tool.to_string();
    let first_tool_task = tokio::spawn(async move { p.mcp_call_tool(&name, json!({})).await });

    let first_response = first_task.await.unwrap();
    assert_eq!(first_response["choices"][0]["finish_reason"], "tool_calls");
    assert_eq!(first_response["choices"][0]["message"]["tool_calls"][0]["function"]["name"], first_tool);

    // codebase sends first tool result
    let after_first_tool = with_tool_result(&base, &first_response, first_return);
    let p = proxy.clone();
    let after = after_first_tool.clone();
    let second_task = tokio::spawn(async move { p.openai_chat(json!({"messages": after})).await });

    assert_eq!(first_tool_task.await.unwrap(), first_return);

    // agent calls second tool
    let p = proxy.clone();
    let name = second_tool.to_string();
    let second_tool_task = tokio::spawn(async move { p.mcp_call_tool(&name, json!({})).await });

    let second_response = second_task.await.unwrap();
    assert_eq!(second_response["choices"][0]["finish_reason"], "tool_calls");
    assert_eq!(second_response["choices"][0]["message"]["tool_calls"][0]["function"]["name"], second_tool);

    // codebase sends second tool result
    let after_second_tool = with_tool_result(&after_first_tool, &second_response, second_return);
    let p = proxy.clone();
    let final_task = tokio::spawn(async move { p.openai_chat(json!({"messages": after_second_tool})).await });

    assert_eq!(second_tool_task.await.unwrap(), second_return);

    proxy.mcp_write_message(target).await;

    let final_response = final_task.await.unwrap();
    assert_eq!(final_response["choices"][0]["message"]["content"], target);
    assert_eq!(final_response["choices"][0]["finish_reason"], "stop");
}

/// Sends all `inputs` concurrently, asserts agent sees them in arrival order.
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

/// After a tool round-trip, asserts read_message returns only `expected_delta`
/// and conversation_id is unchanged.
pub async fn assert_delta_on_continuation(
    proxy: &TestProxy,
    input: Value,
    tool_name: &str,
    tool_return: &str,
    expected_delta: Value,
) {
    let base = input["messages"].as_array().unwrap().clone();
    let p = proxy.clone();
    let openai_task = tokio::spawn(async move { p.openai_chat(input).await });

    let first_read = proxy.mcp_read_message().await;
    let conversation_id = first_read["conversation_id"].clone();

    let p = proxy.clone();
    let name = tool_name.to_string();
    let tool_task = tokio::spawn(async move { p.mcp_call_tool(&name, json!({})).await });

    let tool_call_response = openai_task.await.unwrap();
    let continuation = with_tool_result(&base, &tool_call_response, tool_return);

    let p = proxy.clone();
    tokio::spawn(async move { p.openai_chat(json!({"messages": continuation})).await });

    assert_eq!(tool_task.await.unwrap(), tool_return);

    let delta = proxy.mcp_read_message().await;
    assert_eq!(delta["conversation_id"], conversation_id, "conversation_id changed on continuation");
    assert_eq!(delta["messages"], expected_delta);

    proxy.mcp_write_message("done").await;
}

/// Two independent requests get different conversation_ids.
pub async fn assert_different_conversation_ids(proxy: &TestProxy, first: Value, second: Value) {
    let p = proxy.clone();
    tokio::spawn(async move { p.openai_chat(first).await });
    let first_id = proxy.mcp_read_message().await["conversation_id"].clone();
    proxy.mcp_write_message("reply").await;

    let p = proxy.clone();
    tokio::spawn(async move { p.openai_chat(second).await });
    let second_id = proxy.mcp_read_message().await["conversation_id"].clone();
    proxy.mcp_write_message("reply").await;

    assert_ne!(first_id, second_id);
}

/// A request with a diverging message history is treated as a new conversation.
pub async fn assert_diverging_history_is_new_conversation(proxy: &TestProxy, first: Value, diverging: Value) {
    assert_different_conversation_ids(proxy, first, diverging).await;
}

/// After turn 1 completes, turn 2 exposes only `expected_tools` — no state leakage.
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
