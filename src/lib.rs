mod mcp;
mod openai;
mod state;

use axum::{Router, routing::{post, get}};

pub async fn start(port: u16) -> anyhow::Result<()> {
    let state = state::State::new();

    let app = Router::new()
        .route("/v1/chat/completions", post(openai::chat_completions))
        .route("/mcp", post(mcp::handler))
        .route("/mcp", get(mcp::notifications))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
