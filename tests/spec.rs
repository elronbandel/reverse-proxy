mod common;
use common::{weather_tool, TestProxy};
use serde_json::json;

// Verifies doctrine/mcp/RULES.md:3 and doctrine/queue/RULES.md:3
#[tokio::test]
async fn simple_message_is_exposed_to_agent() {
    let proxy = TestProxy::start().await;

    let openai_request = json!({
        "messages": [{ "role": "user", "content": "What is the capital of France?" }]
    });

    let proxy2 = proxy.clone();
    let openai_task =
        tokio::spawn(async move { proxy2.openai_chat(openai_request).await });

    let read_result = proxy.mcp_call_tool("read_message", json!({})).await;
    assert_eq!(
        read_result,
        json!({
            "conversation_id": read_result["conversation_id"],
            "messages": [{ "role": "user", "content": "What is the capital of France?" }]
        })
    );

    proxy.mcp_call_tool("write_message", json!({ "content": "Paris." })).await;
    let _ = openai_task.await.unwrap();
}

// Verifies doctrine/mcp/RULES.md:1
#[tokio::test]
async fn request_tools_appear_in_mcp_tool_list() {
    let proxy = TestProxy::start().await;

    let proxy2 = proxy.clone();
    tokio::spawn(async move {
        proxy2
            .openai_chat(json!({
                "messages": [{ "role": "user", "content": "Weather?" }],
                "tools": [weather_tool()]
            }))
            .await
    });

    let tools = proxy.mcp_list_tools().await;
    assert_eq!(tools, vec!["read_message", "write_message", "get_weather"]);

    proxy.mcp_call_tool("write_message", json!({ "content": "done" })).await;
}

// Verifies doctrine/openai/RULES.md:3
#[tokio::test]
async fn write_message_returns_openai_text_response() {
    let proxy = TestProxy::start().await;

    let proxy2 = proxy.clone();
    let openai_task = tokio::spawn(async move {
        proxy2
            .openai_chat(json!({
                "messages": [{ "role": "user", "content": "Hi" }]
            }))
            .await
    });

    proxy.mcp_call_tool("read_message", json!({})).await;
    proxy.mcp_call_tool("write_message", json!({ "content": "Hello!" })).await;

    let response = openai_task.await.unwrap();
    assert_eq!(response["choices"][0]["message"]["role"], "assistant");
    assert_eq!(response["choices"][0]["message"]["content"], "Hello!");
    assert_eq!(response["choices"][0]["finish_reason"], "stop");
}

// Verifies doctrine/openai/RULES.md:4 and doctrine/mcp/RULES.md:5
#[tokio::test]
async fn tool_call_blocks_until_codebase_returns_result() {
    let proxy = TestProxy::start().await;

    // Codebase sends initial request
    let proxy2 = proxy.clone();
    let first_openai_task = tokio::spawn(async move {
        proxy2
            .openai_chat(json!({
                "messages": [{ "role": "user", "content": "Weather in Paris?" }],
                "tools": [weather_tool()]
            }))
            .await
    });

    proxy.mcp_call_tool("read_message", json!({})).await;

    // Agent calls the tool — this will block until the codebase returns the result
    let proxy2 = proxy.clone();
    let tool_task = tokio::spawn(async move {
        proxy2
            .mcp_call_tool("get_weather", json!({ "location": "Paris" }))
            .await
    });

    // Codebase receives tool_calls and verifies the response shape
    let tool_call_response = first_openai_task.await.unwrap();
    assert_eq!(tool_call_response["choices"][0]["finish_reason"], "tool_calls");
    assert_eq!(
        tool_call_response["choices"][0]["message"]["tool_calls"][0]["function"]["name"],
        "get_weather"
    );

    // Codebase executes the tool and sends the result back
    let proxy2 = proxy.clone();
    let second_openai_task = tokio::spawn(async move {
        proxy2
            .openai_chat(json!({
                "messages": [
                    { "role": "user",      "content": "Weather in Paris?" },
                    { "role": "assistant", "tool_calls": tool_call_response["choices"][0]["message"]["tool_calls"] },
                    { "role": "tool",      "content": "Sunny, 22°C", "tool_call_id": "1" }
                ]
            }))
            .await
    });

    // MCP tool call unblocks with the tool result
    let tool_result = tool_task.await.unwrap();
    assert_eq!(tool_result, "Sunny, 22°C");

    // Agent writes the final answer
    proxy.mcp_call_tool("write_message", json!({ "content": "It's sunny and 22°C in Paris." })).await;
    let final_response = second_openai_task.await.unwrap();
    assert_eq!(
        final_response["choices"][0]["message"]["content"],
        "It's sunny and 22°C in Paris."
    );
}

// Verifies doctrine/queue/RULES.md:1
#[tokio::test]
async fn requests_are_served_in_arrival_order() {
    let proxy = TestProxy::start().await;

    let proxy2 = proxy.clone();
    let _first = tokio::spawn(async move {
        proxy2
            .openai_chat(json!({
                "messages": [{ "role": "user", "content": "First" }]
            }))
            .await
    });

    let proxy2 = proxy.clone();
    let _second = tokio::spawn(async move {
        proxy2
            .openai_chat(json!({
                "messages": [{ "role": "user", "content": "Second" }]
            }))
            .await
    });

    // Agent sees first request
    let first_read = proxy.mcp_call_tool("read_message", json!({})).await;
    assert_eq!(first_read["messages"][0]["content"], "First");
    proxy.mcp_call_tool("write_message", json!({ "content": "reply" })).await;

    // Agent sees second request only after first is resolved
    let second_read = proxy.mcp_call_tool("read_message", json!({})).await;
    assert_eq!(second_read["messages"][0]["content"], "Second");
    proxy.mcp_call_tool("write_message", json!({ "content": "reply" })).await;
}

// Verifies doctrine/queue/RULES.md:4 and doctrine/mcp/RULES.md:2
#[tokio::test]
async fn read_message_returns_only_delta_on_same_conversation() {
    let proxy = TestProxy::start().await;

    let proxy2 = proxy.clone();
    let first_task = tokio::spawn(async move {
        proxy2
            .openai_chat(json!({
                "messages": [{ "role": "user", "content": "First message" }],
                "tools": [weather_tool()]
            }))
            .await
    });

    let first_read = proxy.mcp_call_tool("read_message", json!({})).await;
    let conversation_id = first_read["conversation_id"].clone();

    let proxy2 = proxy.clone();
    let tool_task = tokio::spawn(async move {
        proxy2.mcp_call_tool("get_weather", json!({ "location": "Paris" })).await
    });

    let tool_call_response = first_task.await.unwrap();

    // Codebase sends back the full history including the tool result
    let proxy2 = proxy.clone();
    tokio::spawn(async move {
        proxy2
            .openai_chat(json!({
                "messages": [
                    { "role": "user",      "content": "First message" },
                    { "role": "assistant", "tool_calls": tool_call_response["choices"][0]["message"]["tool_calls"] },
                    { "role": "tool",      "content": "Sunny, 22°C", "tool_call_id": "1" }
                ]
            }))
            .await
    });

    // MCP tool call resolves with just the tool result, not the full history
    let tool_result = tool_task.await.unwrap();
    assert_eq!(tool_result, "Sunny, 22°C");

    // Same conversation — no tools/list_changed, same conversation_id
    let delta_read = proxy.mcp_call_tool("read_message", json!({})).await;
    assert_eq!(delta_read["conversation_id"], conversation_id);
    assert_eq!(
        delta_read["messages"],
        json!([{ "role": "tool", "content": "Sunny, 22°C", "tool_call_id": "1" }])
    );

    proxy.mcp_call_tool("write_message", json!({ "content": "done" })).await;
}
