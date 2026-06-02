use std::sync::Arc;
use axum::{Json, extract::State, response::sse::{Event, Sse}, response::IntoResponse};
use serde_json::{json, Value};
use crate::state;

pub async fn handler(
    State(state): State<Arc<state::State>>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let id     = body["id"].clone();
    let method = body["method"].as_str().unwrap_or("");
    let params = &body["params"];

    let result = match method {
        "initialize" => {
            json!({
                "protocolVersion": "2025-11-25",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "reverse-proxy", "version": "0.1.0" }
            })
        }
        "tools/list" => {
            let tools = state.list_tools().await;
            json!({ "tools": tools })
        }
        "tools/call" => {
            let name = params["name"].as_str().unwrap_or("");
            let args = params["arguments"].clone();
            match name {
                "read_message" => {
                    let (conversation_id, messages) = state.read_message().await;
                    let payload = json!({ "conversation_id": conversation_id, "messages": messages });
                    json!({ "content": [{ "type": "text", "text": payload.to_string() }] })
                }
                "write_message" => {
                    let content = args["content"].as_str().unwrap_or("").to_string();
                    state.write_message(content).await;
                    json!({ "content": [{ "type": "text", "text": "ok" }] })
                }
                tool => {
                    let rx = state.call_tool(tool.to_string(), args).await;
                    let result = rx.await.unwrap_or_default();
                    json!({ "content": [{ "type": "text", "text": result }] })
                }
            }
        }
        _ => return Json(json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": "Method not found" } })),
    };

    Json(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

/// GET /mcp — persistent SSE stream for notifications (tools/list_changed).
pub async fn notifications(
    State(state): State<Arc<state::State>>,
) -> impl IntoResponse {
    let mut rx = state.subscribe_new_conv();
    let stream = async_stream::stream! {
        loop {
            match rx.changed().await {
                Err(_) => break,
                Ok(()) => {
                    let event = json!({ "jsonrpc": "2.0", "method": "notifications/tools/list_changed", "params": {} });
                    yield Ok::<Event, std::convert::Infallible>(Event::default().data(event.to_string()));
                }
            }
        }
    };
    Sse::new(stream)
}
