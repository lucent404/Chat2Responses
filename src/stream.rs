use async_stream::stream;
use axum::response::{
    sse::{Event, KeepAlive},
    Sse,
};
use eventsource_stream::Eventsource as EventsourceExt;
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::time::{timeout, Duration};
use tracing::{error, warn};

use crate::{
    session::SessionStore,
    types::{ChatMessage, ChatRequest, ChatStreamChunk},
};

pub struct StreamArgs {
    pub client: reqwest::Client,
    pub url: String,
    pub api_key: Arc<String>,
    pub chat_req: ChatRequest,
    pub response_id: String,
    pub sessions: SessionStore,
    /// The fully translated request messages (including replayed history).
    /// Used to save correct session history so turn-level reasoning can be
    /// recovered when Codex replays the conversation without previous_response_id.
    pub request_messages: Vec<ChatMessage>,
    pub model: String,
    pub upstream_timeout_seconds: i64,
    pub on_complete: Option<Arc<dyn Fn(StreamLog) + Send + Sync>>,
}

#[derive(Clone, Debug)]
pub struct StreamLog {
    pub status_code: u16,
    pub error: Option<String>,
}

struct ToolCallAccum {
    id: String,
    name: String,
    arguments: String,
}

/// Translate an upstream Chat Completions SSE stream into a Responses API SSE stream.
///
/// Text response event sequence:
///   response.created → [optional reasoning summary events] →
///   response.output_item.added (message) → response.output_text.delta* →
///   response.output_item.done → response.completed
///
/// Tool call response event sequence:
///   response.created → [accumulate deltas] → response.output_item.added (function_call)
///   → response.function_call_arguments.delta → response.output_item.done → response.completed
pub fn translate_stream(
    args: StreamArgs,
) -> Sse<impl futures_util::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let StreamArgs {
        client,
        url,
        api_key,
        chat_req,
        response_id,
        sessions,
        request_messages,
        model,
        upstream_timeout_seconds,
        on_complete,
    } = args;
    let msg_item_id = format!("msg_{}", uuid::Uuid::new_v4().simple());
    let reasoning_item_id = format!("rs_{}", uuid::Uuid::new_v4().simple());

    let event_stream = stream! {
        yield Ok(Event::default()
            .event("response.created")
            .data(json!({
                "type": "response.created",
                "response": { "id": &response_id, "status": "in_progress", "model": &model }
            }).to_string()));

        let mut builder = client.post(&url).header("Content-Type", "application/json");
        if !api_key.is_empty() {
            builder = builder.bearer_auth(api_key.as_str());
        }
        let send = builder.json(&chat_req).send();
        let upstream_result = if upstream_timeout_seconds > 0 {
            match timeout(Duration::from_secs(upstream_timeout_seconds as u64), send).await {
                Ok(result) => result,
                Err(_) => {
                    let message = "upstream request timed out".to_string();
                    error!("{message}");
                    if let Some(callback) = &on_complete {
                        callback(StreamLog {
                            status_code: 502,
                            error: Some(message.clone()),
                        });
                    }
                    yield Ok(Event::default().event("response.failed").data(
                        json!({"type": "response.failed", "response": {"id": &response_id, "status": "failed", "error": {"code": "connection_timeout", "message": message}}}).to_string()
                    ));
                    return;
                }
            }
        } else {
            send.await
        };

        let upstream = match upstream_result {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                let status = r.status();
                let body = r.text().await.unwrap_or_default();
                error!("upstream {status}: {body}");
                if let Some(callback) = &on_complete {
                    callback(StreamLog {
                        status_code: status.as_u16(),
                        error: Some(body.clone()),
                    });
                }
                yield Ok(Event::default().event("response.failed").data(
                    json!({"type": "response.failed", "response": {"id": &response_id, "status": "failed", "error": {"code": status.as_u16().to_string(), "message": body}}}).to_string()
                ));
                return;
            }
            Err(e) => {
                error!("upstream request failed: {e}");
                if let Some(callback) = &on_complete {
                    callback(StreamLog {
                        status_code: 502,
                        error: Some(e.to_string()),
                    });
                }
                yield Ok(Event::default().event("response.failed").data(
                    json!({"type": "response.failed", "response": {"id": &response_id, "status": "failed", "error": {"code": "connection_error", "message": e.to_string()}}}).to_string()
                ));
                return;
            }
        };

        let mut accumulated_text = String::new();
        let mut accumulated_reasoning = String::new();
        let mut tool_calls: BTreeMap<usize, ToolCallAccum> = BTreeMap::new();
        let mut emitted_message_item = false;
        let mut emitted_reasoning_item = false;
        let mut completed_reasoning_item = false;
        let mut emitted_text_part = false;
        let mut stream_done = false;
        let mut source = upstream.bytes_stream().eventsource();

        while let Some(ev) = source.next().await {
            match ev {
                Err(e) => {
                    warn!("SSE parse error: {e}");
                    break;
                }
                Ok(ev) if ev.data.trim() == "[DONE]" => {
                    stream_done = true;
                    break;
                }
                Ok(ev) if ev.data.is_empty() => continue,
                Ok(ev) => {
                    match serde_json::from_str::<ChatStreamChunk>(&ev.data) {
                        Err(e) => warn!("chunk parse error: {e} — data: {}", ev.data),
                        Ok(chunk) => {
                            for choice in &chunk.choices {
                                // Reasoning/thinking content (kimi-k2.6 etc.)
                                if let Some(rc) = choice.delta.reasoning_content.as_deref() {
                                    if !rc.is_empty() {
                                        if !emitted_reasoning_item {
                                            yield Ok(Event::default()
                                                .event("response.output_item.added")
                                                .data(json!({
                                                    "type": "response.output_item.added",
                                                    "output_index": 0,
                                                    "item": {
                                                        "type": "reasoning",
                                                        "id": &reasoning_item_id,
                                                        "summary": []
                                                    }
                                                }).to_string()));
                                            yield Ok(Event::default()
                                                .event("response.reasoning_summary_part.added")
                                                .data(json!({
                                                    "type": "response.reasoning_summary_part.added",
                                                    "item_id": &reasoning_item_id,
                                                    "output_index": 0,
                                                    "summary_index": 0,
                                                    "part": {
                                                        "type": "summary_text",
                                                        "text": ""
                                                    }
                                                }).to_string()));
                                            emitted_reasoning_item = true;
                                        }
                                        accumulated_reasoning.push_str(rc);
                                        yield Ok(Event::default()
                                            .event("response.reasoning_summary_text.delta")
                                            .data(json!({
                                                "type": "response.reasoning_summary_text.delta",
                                                "item_id": &reasoning_item_id,
                                                "output_index": 0,
                                                "summary_index": 0,
                                                "delta": rc
                                            }).to_string()));
                                    }
                                }

                                // Text content
                                let content = choice.delta.content.as_deref().unwrap_or("");
                                if !content.is_empty() {
                                    let msg_output_index = if emitted_reasoning_item { 1 } else { 0 };
                                    if emitted_reasoning_item && !completed_reasoning_item {
                                        yield Ok(Event::default()
                                            .event("response.reasoning_summary_text.done")
                                            .data(json!({
                                                "type": "response.reasoning_summary_text.done",
                                                "item_id": &reasoning_item_id,
                                                "output_index": 0,
                                                "summary_index": 0,
                                                "text": &accumulated_reasoning
                                            }).to_string()));
                                        yield Ok(Event::default()
                                            .event("response.reasoning_summary_part.done")
                                            .data(json!({
                                                "type": "response.reasoning_summary_part.done",
                                                "item_id": &reasoning_item_id,
                                                "output_index": 0,
                                                "summary_index": 0,
                                                "part": {
                                                    "type": "summary_text",
                                                    "text": &accumulated_reasoning
                                                }
                                            }).to_string()));
                                        yield Ok(Event::default()
                                            .event("response.output_item.done")
                                            .data(json!({
                                                "type": "response.output_item.done",
                                                "output_index": 0,
                                                "item": {
                                                    "type": "reasoning",
                                                    "id": &reasoning_item_id,
                                                    "summary": [{"type": "summary_text", "text": &accumulated_reasoning}]
                                                }
                                            }).to_string()));
                                        completed_reasoning_item = true;
                                    }
                                    if !emitted_message_item {
                                        yield Ok(Event::default()
                                            .event("response.output_item.added")
                                            .data(json!({
                                                "type": "response.output_item.added",
                                                "output_index": msg_output_index,
                                                "item": {
                                                    "type": "message",
                                                    "id": &msg_item_id,
                                                    "role": "assistant",
                                                    "status": "in_progress",
                                                    "content": []
                                                }
                                            }).to_string()));
                                        emitted_message_item = true;
                                    }
                                    if !emitted_text_part {
                                        yield Ok(Event::default()
                                            .event("response.content_part.added")
                                            .data(json!({
                                                "type": "response.content_part.added",
                                                "item_id": &msg_item_id,
                                                "output_index": msg_output_index,
                                                "content_index": 0,
                                                "part": {
                                                    "type": "output_text",
                                                    "text": ""
                                                }
                                            }).to_string()));
                                        emitted_text_part = true;
                                    }
                                    accumulated_text.push_str(content);
                                    yield Ok(Event::default()
                                        .event("response.output_text.delta")
                                        .data(json!({
                                            "type": "response.output_text.delta",
                                            "item_id": &msg_item_id,
                                            "output_index": msg_output_index,
                                            "content_index": 0,
                                            "delta": content
                                        }).to_string()));
                                }

                                // Tool call deltas
                                if let Some(tcs) = &choice.delta.tool_calls {
                                    for tc in tcs {
                                        let entry = tool_calls.entry(tc.index).or_insert_with(|| ToolCallAccum {
                                            id: String::new(),
                                            name: String::new(),
                                            arguments: String::new(),
                                        });
                                        if let Some(id) = &tc.id {
                                            if !id.is_empty() {
                                                entry.id = id.clone();
                                            }
                                        }
                                        if let Some(f) = &tc.function {
                                            if let Some(n) = &f.name {
                                                if !n.is_empty() {
                                                    entry.name.push_str(n);
                                                }
                                            }
                                            if let Some(a) = &f.arguments {
                                                entry.arguments.push_str(a);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if emitted_reasoning_item && !completed_reasoning_item {
            yield Ok(Event::default()
                .event("response.reasoning_summary_text.done")
                .data(json!({
                    "type": "response.reasoning_summary_text.done",
                    "item_id": &reasoning_item_id,
                    "output_index": 0,
                    "summary_index": 0,
                    "text": &accumulated_reasoning
                }).to_string()));
            yield Ok(Event::default()
                .event("response.reasoning_summary_part.done")
                .data(json!({
                    "type": "response.reasoning_summary_part.done",
                    "item_id": &reasoning_item_id,
                    "output_index": 0,
                    "summary_index": 0,
                    "part": {
                        "type": "summary_text",
                        "text": &accumulated_reasoning
                    }
                }).to_string()));
            yield Ok(Event::default()
                .event("response.output_item.done")
                .data(json!({
                    "type": "response.output_item.done",
                    "output_index": 0,
                    "item": {
                        "type": "reasoning",
                        "id": &reasoning_item_id,
                        "summary": [{"type": "summary_text", "text": &accumulated_reasoning}]
                    }
                }).to_string()));
        }

        if let Some(msg_item_id) = (emitted_message_item).then(|| msg_item_id.clone()) {
            let msg_output_index = if emitted_reasoning_item { 1 } else { 0 };
            if emitted_text_part {
                yield Ok(Event::default()
                    .event("response.output_text.done")
                    .data(json!({
                        "type": "response.output_text.done",
                        "item_id": &msg_item_id,
                        "output_index": msg_output_index,
                        "content_index": 0,
                        "text": &accumulated_text
                    }).to_string()));
                yield Ok(Event::default()
                    .event("response.content_part.done")
                    .data(json!({
                        "type": "response.content_part.done",
                        "item_id": &msg_item_id,
                        "output_index": msg_output_index,
                        "content_index": 0,
                        "part": {
                            "type": "output_text",
                            "text": &accumulated_text
                        }
                    }).to_string()));
            }
            yield Ok(Event::default()
                .event("response.output_item.done")
                .data(json!({
                    "type": "response.output_item.done",
                    "output_index": msg_output_index,
                    "item": {
                        "type": "message",
                        "id": &msg_item_id,
                        "role": "assistant",
                        "status": "completed",
                        "content": [{"type": "output_text", "text": &accumulated_text}]
                    }
                }).to_string()));
        }

        // Emit function_call items for each accumulated tool call
        let base_index: usize = usize::from(emitted_reasoning_item) + usize::from(emitted_message_item);
        let mut fc_items: Vec<Value> = Vec::new();

        for (rel_idx, (_, tc)) in tool_calls.iter().enumerate() {
            let fc_item_id = format!("fc_{}", uuid::Uuid::new_v4().simple());
            let output_index = base_index + rel_idx;

            yield Ok(Event::default()
                .event("response.output_item.added")
                .data(json!({
                    "type": "response.output_item.added",
                    "output_index": output_index,
                    "item": {
                        "type": "function_call",
                        "id": &fc_item_id,
                        "call_id": &tc.id,
                        "name": &tc.name,
                        "arguments": "",
                        "status": "in_progress"
                    }
                }).to_string()));

            if !tc.arguments.is_empty() {
                yield Ok(Event::default()
                    .event("response.function_call_arguments.delta")
                    .data(json!({
                        "type": "response.function_call_arguments.delta",
                        "item_id": &fc_item_id,
                        "output_index": output_index,
                        "delta": &tc.arguments
                    }).to_string()));
            }

            yield Ok(Event::default()
                .event("response.output_item.done")
                .data(json!({
                    "type": "response.output_item.done",
                    "output_index": output_index,
                    "item": {
                        "type": "function_call",
                        "id": &fc_item_id,
                        "call_id": &tc.id,
                        "name": &tc.name,
                        "arguments": &tc.arguments,
                        "status": "completed"
                    }
                }).to_string()));

            fc_items.push(json!({
                "type": "function_call",
                "id": fc_item_id,
                "call_id": &tc.id,
                "name": &tc.name,
                "arguments": &tc.arguments,
                "status": "completed"
            }));
        }

        if stream_done {
            // Persist turn to session store
            // Store reasoning_content per call_id so translate.rs can inject it
            // back when Codex replays function_call items in the next request.
            for tc in tool_calls.values() {
                if !tc.id.is_empty() {
                    sessions.store_reasoning(tc.id.clone(), accumulated_reasoning.clone());
                }
            }

            let assistant_tool_calls: Option<Vec<Value>> = if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls.values().map(|tc| json!({
                    "id": &tc.id,
                    "type": "function",
                    "function": { "name": &tc.name, "arguments": &tc.arguments }
                })).collect())
            };
            let assistant_msg = ChatMessage {
                role: "assistant".into(),
                content: if accumulated_text.is_empty() { None } else { Some(serde_json::Value::String(accumulated_text.clone())) },
                reasoning_content: if accumulated_reasoning.is_empty() { None } else { Some(accumulated_reasoning.clone()) },
                tool_calls: assistant_tool_calls,
                tool_call_id: None,
                name: None,
            };

            // Index reasoning by turn fingerprint so it can be recovered when
            // Codex replays the full conversation in input[] without previous_response_id.
            if !accumulated_reasoning.is_empty() {
                sessions.store_turn_reasoning(&request_messages, &assistant_msg, accumulated_reasoning.clone());
            }

            // Save the full request conversation (including current input items)
            // so that history is complete for the next turn.
            let mut messages = request_messages;
            messages.push(assistant_msg);
            sessions.save_with_id(response_id.clone(), messages);

            // Build output array for response.completed
            let mut output_items: Vec<Value> = Vec::new();
            if emitted_reasoning_item {
                output_items.push(json!({
                    "type": "reasoning",
                    "id": &reasoning_item_id,
                    "summary": [{"type": "summary_text", "text": &accumulated_reasoning}]
                }));
            }
            if emitted_message_item {
                output_items.push(json!({
                    "type": "message",
                    "id": &msg_item_id,
                    "role": "assistant",
                    "status": "completed",
                    "content": [{"type": "output_text", "text": &accumulated_text}]
                }));
            }
            output_items.extend(fc_items);

            yield Ok(Event::default()
                .event("response.completed")
                .data(json!({
                    "type": "response.completed",
                    "response": {
                        "id": &response_id,
                        "status": "completed",
                        "model": &model,
                        "output": output_items
                    }
                }).to_string()));
            if let Some(callback) = &on_complete {
                callback(StreamLog {
                    status_code: 200,
                    error: None,
                });
            }
        } else {
            // Stream did not complete cleanly: do NOT save session state
            // to avoid creating an assistant-with-tool_calls gap in history
            // that causes upstream "insufficient tool messages" errors.
            warn!("stream disconnected before [DONE] — discarding partial turn");
            if let Some(callback) = &on_complete {
                callback(StreamLog {
                    status_code: 502,
                    error: Some("stream disconnected before completion".into()),
                });
            }
            yield Ok(Event::default()
                .event("response.failed")
                .data(json!({
                    "type": "response.failed",
                    "response": {
                        "id": &response_id,
                        "status": "failed",
                        "error": {
                            "code": "stream_incomplete",
                            "message": "stream disconnected before completion"
                        }
                    }
                }).to_string()));
        }
    };

    Sse::new(event_stream).keep_alive(KeepAlive::default())
}
