mod common;
use common::*;
use serde_json::json;
use tokio::time::Duration;

// ── mcp/RULES.md:3, queue/RULES.md:3 ──────────────────────────────────────

#[tokio::test]
async fn simple_message_is_exposed_to_agent() {
    let proxy = TestProxy::start().await;

    let input = json!({
        "messages": [{ "role": "user", "content": "What is the capital of France?" }]
    });
    let target = "Paris.";

    assert_text_reply(&proxy, input, target).await;
}

// ── mcp/RULES.md:1 — with tools ───────────────────────────────────────────

#[tokio::test]
async fn request_tools_appear_in_mcp_tool_list() {
    let proxy = TestProxy::start().await;

    let input = json!({
        "messages": [{ "role": "user", "content": "Weather?" }],
        "tools": [{
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get current weather",
                "parameters": { "type": "object", "properties": { "location": { "type": "string" } } }
            }
        }]
    });
    let target = &["read_message", "write_message", "get_weather"];

    assert_mcp_tool_list(&proxy, input, target).await;
}

// ── mcp/RULES.md:1 — without tools ────────────────────────────────────────

#[tokio::test]
async fn no_tools_request_exposes_only_fixed_tools() {
    let proxy = TestProxy::start().await;

    let input = json!({
        "messages": [{ "role": "user", "content": "Hello" }]
    });
    let target = &["read_message", "write_message"];

    assert_mcp_tool_list(&proxy, input, target).await;
}

// ── openai/RULES.md:3 ─────────────────────────────────────────────────────

#[tokio::test]
async fn write_message_returns_stop_response() {
    let proxy = TestProxy::start().await;

    let input = json!({
        "messages": [{ "role": "user", "content": "Hi" }]
    });
    let target = "Hello!";

    assert_text_reply(&proxy, input, target).await;
}

// ── openai/RULES.md:4, mcp/RULES.md:5 ────────────────────────────────────

#[tokio::test]
async fn tool_call_blocks_until_codebase_returns_result() {
    let proxy = TestProxy::start().await;

    let input = json!({
        "messages": [{ "role": "user", "content": "Weather in Paris?" }],
        "tools": [{
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get current weather",
                "parameters": { "type": "object", "properties": { "location": { "type": "string" } } }
            }
        }]
    });
    let tool_return = "Sunny, 22°C";
    let target     = "It's sunny and 22°C in Paris.";

    assert_tool_round_trip(&proxy, input, "get_weather", tool_return, target).await;
}

// ── queue/RULES.md:1 ──────────────────────────────────────────────────────

#[tokio::test]
async fn requests_are_served_in_arrival_order() {
    let proxy = TestProxy::start().await;

    let inputs = vec![
        json!({ "messages": [{ "role": "user", "content": "First"  }] }),
        json!({ "messages": [{ "role": "user", "content": "Second" }] }),
    ];

    assert_fifo_order(&proxy, inputs).await;
}

// ── queue/RULES.md:4, queue/RULES.md:6, mcp/RULES.md:2 ───────────────────

#[tokio::test]
async fn read_message_returns_only_delta_on_continuation() {
    let proxy = TestProxy::start().await;

    let input = json!({
        "messages": [{ "role": "user", "content": "Weather in Paris?" }],
        "tools": [{
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get current weather",
                "parameters": { "type": "object", "properties": { "location": { "type": "string" } } }
            }
        }]
    });
    let expected_delta = json!([
        { "role": "tool", "content": "Sunny, 22°C", "tool_call_id": "1" }
    ]);

    assert_delta_on_continuation(&proxy, input, "get_weather", "Sunny, 22°C", expected_delta).await;
}

// ── queue/RULES.md:3, queue/RULES.md:5 ────────────────────────────────────

#[tokio::test]
async fn new_conversation_gets_different_conversation_id() {
    let proxy = TestProxy::start().await;

    let first  = json!({ "messages": [{ "role": "user", "content": "Hello"   }] });
    let second = json!({ "messages": [{ "role": "user", "content": "Goodbye" }] });

    assert_different_conversation_ids(&proxy, first, second).await;
}

// ── proxy/RULES.md:3 ──────────────────────────────────────────────────────

#[tokio::test]
async fn tools_from_previous_turn_do_not_leak_into_next() {
    let proxy = TestProxy::start().await;

    let first = json!({
        "messages": [{ "role": "user", "content": "First" }],
        "tools": [{
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get current weather",
                "parameters": { "type": "object", "properties": { "location": { "type": "string" } } }
            }
        }]
    });
    let second = json!({
        "messages": [{ "role": "user", "content": "Second" }]
    });

    assert_no_state_leakage(&proxy, first, second, &["read_message", "write_message"]).await;
}

// ── proxy/RULES.md:4 ──────────────────────────────────────────────────────

#[tokio::test]
async fn server_accepts_new_requests_while_turn_is_in_progress() {
    let proxy = TestProxy::start().await;
    assert_concurrent_ingestion(&proxy, 3).await;
}

// ── mcp/RULES.md:2 ────────────────────────────────────────────────────────

#[tokio::test]
async fn tools_list_changed_fires_on_new_conversation_not_on_continuation() {
    let proxy = TestProxy::start().await;
    let mut notifications = proxy.mcp_subscribe_notifications().await;

    // new conversation → notification expected
    let p = proxy.clone();
    tokio::spawn(async move {
        p.openai_chat(json!({
            "messages": [{ "role": "user", "content": "Weather?" }],
            "tools": [{
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get current weather",
                    "parameters": { "type": "object", "properties": { "location": { "type": "string" } } }
                }
            }]
        }))
        .await
    });
    assert_eq!(notifications.recv().await.unwrap(), "notifications/tools/list_changed");
    proxy.mcp_write_message("done").await;

    // continuation → no notification within 50 ms
    let p = proxy.clone();
    tokio::spawn(async move { p.mcp_call_tool("get_weather", json!({})).await });
    assert!(
        tokio::time::timeout(Duration::from_millis(50), notifications.recv())
            .await
            .is_err(),
        "unexpected tools/list_changed on continuation"
    );
}
