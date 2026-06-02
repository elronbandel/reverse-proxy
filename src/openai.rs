use std::sync::Arc;
use axum::{Json, extract::State};
use serde_json::Value;
use crate::state;

pub async fn chat_completions(
    State(state): State<Arc<state::State>>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let messages = body["messages"].as_array().cloned().unwrap_or_default();
    let tools    = body["tools"].as_array().cloned().unwrap_or_default();
    let rx = state.push(messages, tools).await;
    Json(rx.await.unwrap_or_else(|_| Value::Null))
}
