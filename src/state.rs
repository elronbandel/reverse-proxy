use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify, oneshot, watch};
use serde_json::Value;
use uuid::Uuid;

pub struct State {
    inner:       Mutex<Inner>,
    turn_ready:  Notify,
    new_conv_tx: watch::Sender<u64>,
}

struct Inner {
    queue:                  VecDeque<Pending>,
    active:                 Option<Turn>,
    last_snapshot:          Vec<Value>,
    last_conversation_id:   Option<String>,
}

struct Pending {
    messages:   Vec<Value>,
    tools:      Vec<Value>,
    respond:    oneshot::Sender<Value>,
}

struct Turn {
    id:             String,
    messages:       Vec<Value>,
    snapshot:       Vec<Value>,
    tools:          Vec<Value>,
    respond:        Option<oneshot::Sender<Value>>,
    tool_result_tx: Option<oneshot::Sender<String>>,
}

impl State {
    pub fn new() -> Arc<Self> {
        let (new_conv_tx, _) = watch::channel(0u64);
        Arc::new(Self {
            inner: Mutex::new(Inner {
                queue:                VecDeque::new(),
                active:               None,
                last_snapshot:        vec![],
                last_conversation_id: None,
            }),
            turn_ready:  Notify::new(),
            new_conv_tx,
        })
    }

    pub async fn push(&self, messages: Vec<Value>, tools: Vec<Value>) -> oneshot::Receiver<Value> {
        let (respond_tx, respond_rx) = oneshot::channel();

        let mut inner = self.inner.lock().await;

        // Tool result: active turn is waiting and this request continues the conversation
        if let Some(turn) = &mut inner.active {
            if turn.tool_result_tx.is_some() && messages.starts_with(&turn.snapshot) {
                let content = messages.last()
                    .and_then(|m| m["content"].as_str())
                    .unwrap_or("")
                    .to_string();
                let _ = turn.tool_result_tx.take().unwrap().send(content);
                // snapshot = everything except the tool result so delta = [tool_result]
                turn.snapshot = messages[..messages.len() - 1].to_vec();
                turn.messages = messages;
                turn.respond  = Some(respond_tx);
                drop(inner);
                self.turn_ready.notify_one();
                return respond_rx;
            }
        }

        let was_idle = inner.active.is_none();
        inner.queue.push_back(Pending { messages, tools, respond: respond_tx });
        drop(inner);

        if was_idle {
            self.advance().await;
        }
        respond_rx
    }

    async fn advance(&self) {
        let is_new = {
            let mut inner = self.inner.lock().await;
            if inner.active.is_some() || inner.queue.is_empty() {
                return;
            }
            let pending = inner.queue.pop_front().unwrap();
            let is_continuation = !inner.last_snapshot.is_empty()
                && pending.messages.starts_with(&inner.last_snapshot);
            let id = if is_continuation {
                inner.last_conversation_id.clone().unwrap_or_else(|| Uuid::new_v4().to_string())
            } else {
                Uuid::new_v4().to_string()
            };
            inner.last_conversation_id = Some(id.clone());
            inner.active = Some(Turn {
                id,
                snapshot: if is_continuation { inner.last_snapshot.clone() } else { vec![] },
                messages: pending.messages,
                tools:    pending.tools,
                respond:  Some(pending.respond),
                tool_result_tx: None,
            });
            !is_continuation
        };
        self.turn_ready.notify_one();
        if is_new {
            let next = *self.new_conv_tx.borrow() + 1;
            let _ = self.new_conv_tx.send(next);
        }
    }

    pub async fn read_message(&self) -> (String, Vec<Value>) {
        loop {
            let notified = self.turn_ready.notified();
            tokio::pin!(notified);
            {
                let mut inner = self.inner.lock().await;
                if let Some(turn) = &mut inner.active {
                    let delta: Vec<Value> = turn.messages[turn.snapshot.len()..].to_vec();
                    if !delta.is_empty() {
                        let id = turn.id.clone();
                        turn.snapshot = turn.messages.clone();
                        return (id, delta);
                    }
                }
            }
            notified.await;
        }
    }

    pub async fn write_message(&self, content: String) {
        let respond = {
            let mut inner = self.inner.lock().await;
            let turn = inner.active.take().unwrap();
            inner.last_snapshot = turn.messages.clone();
            turn.respond.unwrap()
        };
        let _ = respond.send(serde_json::json!({
            "choices": [{ "message": { "role": "assistant", "content": content }, "finish_reason": "stop" }]
        }));
        self.advance().await;
    }

    pub async fn call_tool(&self, name: String, args: Value) -> oneshot::Receiver<String> {
        let (tool_result_tx, tool_result_rx) = oneshot::channel::<String>();
        let respond = {
            let mut inner = self.inner.lock().await;
            let Some(turn) = inner.active.as_mut() else { return tool_result_rx };
            turn.tool_result_tx = Some(tool_result_tx);
            turn.respond.take().unwrap()
        };
        let call_id = Uuid::new_v4().to_string();
        let _ = respond.send(serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{ "id": call_id, "type": "function", "function": { "name": name, "arguments": args.to_string() } }]
                },
                "finish_reason": "tool_calls"
            }]
        }));
        tool_result_rx
    }

    pub async fn list_tools(&self) -> Vec<Value> {
        let inner = self.inner.lock().await;
        let mut tools = vec![
            serde_json::json!({ "name": "read_message",  "description": "Read the next queued message",  "inputSchema": { "type": "object", "properties": {} } }),
            serde_json::json!({ "name": "write_message", "description": "Send a response to the caller", "inputSchema": { "type": "object", "properties": { "content": { "type": "string" } }, "required": ["content"] } }),
        ];
        if let Some(turn) = &inner.active {
            for tool in &turn.tools {
                let f = &tool["function"];
                tools.push(serde_json::json!({
                    "name":        f["name"],
                    "description": f["description"],
                    "inputSchema": f["parameters"]
                }));
            }
        }
        tools
    }

    pub fn subscribe_new_conv(&self) -> watch::Receiver<u64> {
        self.new_conv_tx.subscribe()
    }
}
