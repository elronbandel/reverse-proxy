use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use axum::{extract::State, routing::post, Json, Router};
use reqwest::Client;
use serde_json::{json, Value};
use tokio::sync::{oneshot, Mutex, Notify};
use tokio::time::{sleep, Duration};

// ── Fake proxy server ──────────────────────────────────────────────────────

static CONV_COUNTER: AtomicU64 = AtomicU64::new(0);

fn new_conv_id() -> String {
    format!("conv-{}", CONV_COUNTER.fetch_add(1, Ordering::Relaxed))
}

struct QueuedRequest {
    conversation_id: String,
    messages: Value,
    tools: Vec<Value>,
    reply_tx: oneshot::Sender<Value>,
}

struct FakeState {
    queue:        Mutex<VecDeque<QueuedRequest>>,
    active_tools: Mutex<Vec<Value>>,
    active_conv:  Mutex<String>,
    reply_tx:     Mutex<Option<oneshot::Sender<Value>>>,
    tool_result:  Mutex<Option<oneshot::Sender<String>>>,
    // delta cached from a tool-result continuation for the next read_message call
    delta_cache:  Mutex<Option<(String, Value)>>,
    notify:       Notify,
}

impl FakeState {
    fn new() -> Self {
        Self {
            queue:        Mutex::new(VecDeque::new()),
            active_tools: Mutex::new(vec![]),
            active_conv:  Mutex::new(String::new()),
            reply_tx:     Mutex::new(None),
            tool_result:  Mutex::new(None),
            delta_cache:  Mutex::new(None),
            notify:       Notify::new(),
        }
    }
}

async fn openai_handler(State(s): State<Arc<FakeState>>, Json(body): Json<Value>) -> Json<Value> {
    let (reply_tx, reply_rx) = oneshot::channel::<Value>();
    let tool_result_tx = s.tool_result.lock().await.take();

    if let Some(tx) = tool_result_tx {
        // continuation: codebase is returning a tool result
        let last    = body["messages"].as_array().unwrap().last().unwrap().clone();
        let conv_id = s.active_conv.lock().await.clone();
        tx.send(last["content"].as_str().unwrap_or("").to_string()).ok();
        // pre-set reply_tx so the agent can make another tool call without read_message
        *s.reply_tx.lock().await    = Some(reply_tx);
        // cache the delta so read_message can return it if called
        *s.delta_cache.lock().await = Some((conv_id, json!([last])));
        s.notify.notify_one();
    } else {
        // new conversation
        let tools: Vec<Value> = body["tools"].as_array().cloned().unwrap_or_default();
        s.queue.lock().await.push_back(QueuedRequest {
            conversation_id: new_conv_id(),
            messages: body["messages"].clone(),
            tools,
            reply_tx,
        });
    }

    s.notify.notify_one();
    Json(reply_rx.await.unwrap_or_else(|_| json!({})))
}

async fn mcp_handler(State(s): State<Arc<FakeState>>, Json(body): Json<Value>) -> Json<Value> {
    let method = body["method"].as_str().unwrap_or("");
    let params = &body["params"];
    let result = match method {
        "tools/list" => mcp_list(&s).await,
        "tools/call" => match params["name"].as_str().unwrap_or("") {
            "read_message" => mcp_read(&s).await,
            "write_message" => mcp_write(&s, params["arguments"]["content"].as_str().unwrap_or("")).await,
            tool => mcp_tool(&s, tool).await,
        },
        _ => json!(null),
    };
    Json(json!({ "jsonrpc": "2.0", "id": body["id"], "result": result }))
}

async fn mcp_list(s: &FakeState) -> Value {
    let dynamic = {
        let active = s.active_tools.lock().await.clone();
        if !active.is_empty() {
            active
        } else {
            s.queue.lock().await.front().map(|r| r.tools.clone()).unwrap_or_default()
        }
    };
    let mut tools = vec![
        json!({"name":"read_message",  "description":"Read pending message", "input_schema":{}}),
        json!({"name":"write_message", "description":"Write a response",     "input_schema":{"type":"object","properties":{"content":{"type":"string"}}}}),
    ];
    for t in &dynamic {
        let f = &t["function"];
        tools.push(json!({"name": f["name"], "description": f["description"], "input_schema": f["parameters"]}));
    }
    json!({ "tools": tools })
}

async fn mcp_read(s: &FakeState) -> Value {
    // check delta cache first (populated after a tool-result continuation)
    if let Some((conv_id, messages)) = s.delta_cache.lock().await.take() {
        *s.active_conv.lock().await = conv_id.clone();
        let payload = json!({ "conversation_id": conv_id, "messages": messages });
        return json!({"content": [{"type":"text","text": payload.to_string()}]});
    }
    // otherwise wait for a new request in the queue
    loop {
        let req = s.queue.lock().await.pop_front();
        if let Some(req) = req {
            *s.active_tools.lock().await = req.tools;
            *s.active_conv.lock().await  = req.conversation_id.clone();
            *s.reply_tx.lock().await     = Some(req.reply_tx);
            let payload = json!({ "conversation_id": req.conversation_id, "messages": req.messages });
            return json!({"content": [{"type":"text","text": payload.to_string()}]});
        }
        s.notify.notified().await;
    }
}

async fn mcp_write(s: &FakeState, content: &str) -> Value {
    if let Some(tx) = s.reply_tx.lock().await.take() {
        tx.send(json!({"choices":[{"message":{"role":"assistant","content":content},"finish_reason":"stop"}]})).ok();
    }
    *s.active_tools.lock().await = vec![];
    s.notify.notify_one();
    text("ok")
}

async fn mcp_tool(s: &FakeState, tool_name: &str) -> Value {
    let (result_tx, result_rx) = oneshot::channel::<String>();
    *s.tool_result.lock().await = Some(result_tx);
    if let Some(tx) = s.reply_tx.lock().await.take() {
        tx.send(json!({"choices":[{"message":{"role":"assistant","content":null,"tool_calls":[{"id":"call-1","type":"function","function":{"name":tool_name,"arguments":"{}"}}]},"finish_reason":"tool_calls"}]})).ok();
    }
    let result = result_rx.await.unwrap_or_default();
    text(&result)
}

fn text(s: &str) -> Value {
    json!({ "content": [{ "type": "text", "text": s }] })
}

// ── TestProxy ──────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct TestProxy {
    pub openai_url: String,
    pub mcp_url: String,
    client: Client,
}

impl TestProxy {
    pub async fn start() -> Self {
        let state = Arc::new(FakeState::new());

        let openai_app = Router::new().route("/v1/chat/completions", post(openai_handler)).with_state(state.clone());
        let mcp_app    = Router::new().route("/mcp", post(mcp_handler)).with_state(state.clone());

        let openai_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mcp_listener    = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let openai_port = openai_listener.local_addr().unwrap().port();
        let mcp_port    = mcp_listener.local_addr().unwrap().port();

        tokio::spawn(async move { axum::serve(openai_listener, openai_app).await.unwrap() });
        tokio::spawn(async move { axum::serve(mcp_listener,    mcp_app).await.unwrap() });

        TestProxy {
            openai_url: format!("http://127.0.0.1:{openai_port}"),
            mcp_url:    format!("http://127.0.0.1:{mcp_port}"),
            client: Client::new(),
        }
    }

    pub async fn openai_chat(&self, body: Value) -> Value {
        self.client.post(format!("{}/v1/chat/completions", self.openai_url))
            .json(&body).send().await.unwrap().json().await.unwrap()
    }

    pub async fn mcp_list_tools(&self) -> Vec<String> {
        let result = self.mcp_call("tools/list", json!({})).await;
        result["tools"].as_array().unwrap()
            .iter().map(|t| t["name"].as_str().unwrap().to_string()).collect()
    }

    pub async fn mcp_get_tool_definition(&self, name: &str) -> Value {
        let result = self.mcp_call("tools/list", json!({})).await;
        result["tools"].as_array().unwrap()
            .iter().find(|t| t["name"] == name).expect("tool not found").clone()
    }

    pub async fn mcp_read_message(&self) -> Value {
        let result = self.mcp_call("tools/call", json!({"name":"read_message","arguments":{}})).await;
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap()
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
        let body: Value = self.client.post(format!("{}/mcp", self.mcp_url))
            .json(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}))
            .send().await.unwrap().json().await.unwrap();
        body["result"].clone()
    }
}

// ── Shared utility ─────────────────────────────────────────────────────────

fn with_tool_result(history: &[Value], tool_call_response: &Value, tool_return: &str) -> Vec<Value> {
    let tc = &tool_call_response["choices"][0]["message"]["tool_calls"][0];
    let mut next = history.to_vec();
    next.push(tool_call_response["choices"][0]["message"].clone());
    next.push(json!({"role":"tool","content":tool_return,"tool_call_id":tc["id"]}));
    next
}

// ── Assertion helpers ──────────────────────────────────────────────────────

pub async fn assert_text_reply(proxy: &TestProxy, input: Value, target: &str) {
    let p = proxy.clone();
    let openai_task = tokio::spawn(async move { p.openai_chat(input).await });
    proxy.mcp_read_message().await;
    proxy.mcp_write_message(target).await;
    let response = openai_task.await.unwrap();
    assert_eq!(response["choices"][0]["message"]["role"],    "assistant");
    assert_eq!(response["choices"][0]["message"]["content"], target);
    assert_eq!(response["choices"][0]["finish_reason"],      "stop");
}

pub async fn assert_mcp_tool_list(proxy: &TestProxy, input: Value, target: &[&str]) {
    let p = proxy.clone();
    tokio::spawn(async move { p.openai_chat(input).await });
    sleep(Duration::from_millis(10)).await;
    assert_eq!(proxy.mcp_list_tools().await, target);
    proxy.mcp_read_message().await;
    proxy.mcp_write_message("done").await;
}

pub async fn assert_dynamic_tool_schema(proxy: &TestProxy, input: Value, tool_name: &str, target: Value) {
    let p = proxy.clone();
    tokio::spawn(async move { p.openai_chat(input).await });
    sleep(Duration::from_millis(10)).await;
    assert_eq!(proxy.mcp_get_tool_definition(tool_name).await, target);
    proxy.mcp_read_message().await;
    proxy.mcp_write_message("done").await;
}

pub async fn assert_tool_round_trip(
    proxy: &TestProxy, input: Value,
    tool_name: &str, tool_return: &str, target: &str,
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
    proxy.mcp_read_message().await;
    proxy.mcp_write_message(target).await;

    let final_response = final_task.await.unwrap();
    assert_eq!(final_response["choices"][0]["message"]["content"], target);
    assert_eq!(final_response["choices"][0]["finish_reason"],      "stop");
}

pub async fn assert_two_tool_calls_in_one_turn(
    proxy: &TestProxy, input: Value,
    first_tool: &str,  first_return: &str,
    second_tool: &str, second_return: &str,
    target: &str,
) {
    let base = input["messages"].as_array().unwrap().clone();
    let p = proxy.clone();
    let first_task = tokio::spawn(async move { p.openai_chat(input).await });
    proxy.mcp_read_message().await;

    let p = proxy.clone();
    let name = first_tool.to_string();
    let first_tool_task = tokio::spawn(async move { p.mcp_call_tool(&name, json!({})).await });

    let first_response = first_task.await.unwrap();
    assert_eq!(first_response["choices"][0]["finish_reason"], "tool_calls");
    assert_eq!(first_response["choices"][0]["message"]["tool_calls"][0]["function"]["name"], first_tool);

    let after_first = with_tool_result(&base, &first_response, first_return);
    let p = proxy.clone();
    let after = after_first.clone();
    let second_task = tokio::spawn(async move { p.openai_chat(json!({"messages": after})).await });

    assert_eq!(first_tool_task.await.unwrap(), first_return);

    let p = proxy.clone();
    let name = second_tool.to_string();
    let second_tool_task = tokio::spawn(async move { p.mcp_call_tool(&name, json!({})).await });

    let second_response = second_task.await.unwrap();
    assert_eq!(second_response["choices"][0]["finish_reason"], "tool_calls");
    assert_eq!(second_response["choices"][0]["message"]["tool_calls"][0]["function"]["name"], second_tool);

    let after_second = with_tool_result(&after_first, &second_response, second_return);
    let p = proxy.clone();
    let final_task = tokio::spawn(async move { p.openai_chat(json!({"messages": after_second})).await });

    assert_eq!(second_tool_task.await.unwrap(), second_return);
    proxy.mcp_read_message().await;
    proxy.mcp_write_message(target).await;

    let final_response = final_task.await.unwrap();
    assert_eq!(final_response["choices"][0]["message"]["content"], target);
    assert_eq!(final_response["choices"][0]["finish_reason"],      "stop");
}

pub async fn assert_fifo_order(proxy: &TestProxy, inputs: Vec<Value>) {
    let expected: Vec<String> = inputs.iter()
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

pub async fn assert_delta_on_continuation(
    proxy: &TestProxy, input: Value,
    tool_name: &str, tool_return: &str, expected_delta: Value,
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
    // compare role and content, not tool_call_id (implementation detail)
    for (actual, expected) in delta["messages"].as_array().unwrap().iter()
        .zip(expected_delta.as_array().unwrap())
    {
        assert_eq!(actual["role"],    expected["role"]);
        assert_eq!(actual["content"], expected["content"]);
    }

    proxy.mcp_write_message("done").await;
}

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

pub async fn assert_diverging_history_is_new_conversation(proxy: &TestProxy, first: Value, diverging: Value) {
    assert_different_conversation_ids(proxy, first, diverging).await;
}

pub async fn assert_no_state_leakage(
    proxy: &TestProxy,
    first_input: Value, second_input: Value,
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
    proxy.mcp_read_message().await;
    proxy.mcp_write_message("done").await;
}

pub async fn assert_concurrent_ingestion(proxy: &TestProxy, count: usize) {
    let handles: Vec<_> = (0..count).map(|i| {
        let p = proxy.clone();
        tokio::spawn(async move {
            p.openai_chat(json!({"messages":[{"role":"user","content":format!("message {i}")}]})).await
        })
    }).collect();
    for _ in 0..count {
        proxy.mcp_read_message().await;
        proxy.mcp_write_message("reply").await;
    }
    for h in handles { h.await.unwrap(); }
}
