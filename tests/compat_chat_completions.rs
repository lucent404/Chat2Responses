use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{header, StatusCode},
    response::Response,
    Router,
};
use chat2responses::{
    db::{ApiKeyInput, Db, ModelRouteInput, UpstreamInput, UpstreamModelInput},
    security::Crypto,
};
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::{
    net::TcpListener,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const RELAY_BIN: &str = env!("CARGO_BIN_EXE_chat2responses");
const CLIENT_KEY: &str = "service-key";
const PROVIDER_KEY: &str = "provider-key";

fn pick_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

struct Relay {
    child: Child,
    port: u16,
    db_path: std::path::PathBuf,
}

impl Drop for Relay {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.db_path);
    }
}

impl Relay {
    async fn spawn(upstream_base: &str) -> Self {
        let db_path = std::env::temp_dir().join(format!(
            "chat2responses-chat-test-{}.db",
            uuid::Uuid::new_v4().simple()
        ));
        let database_url = format!("sqlite://{}", db_path.display());
        let db = Db::connect(&database_url).await.unwrap();
        let crypto = Crypto::new("test-secret");
        seed_route(&db, &crypto, upstream_base).await;
        drop(db);

        let port = pick_port();
        let child = Command::new(RELAY_BIN)
            .env("CHAT2RESPONSES_PORT", port.to_string())
            .env("CHAT2RESPONSES_DATABASE_URL", &database_url)
            .env("CHAT2RESPONSES_SECRET", "test-secret")
            .env("RUST_LOG", "chat2responses=warn")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn Chat2Responses");

        let mut relay = Self {
            child,
            port,
            db_path,
        };
        relay.wait_ready();
        relay
    }

    fn wait_ready(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(8);
        while Instant::now() < deadline {
            if std::net::TcpStream::connect(("127.0.0.1", self.port)).is_ok() {
                return;
            }
            std::thread::sleep(Duration::from_millis(80));
        }
        panic!("relay did not become ready on :{}", self.port);
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", self.port, path)
    }
}

async fn seed_route(db: &Db, crypto: &Crypto, upstream_base: &str) {
    let upstream = db
        .create_upstream(
            &UpstreamInput {
                name: "test-upstream".into(),
                base_url: upstream_base.into(),
                api_key: PROVIDER_KEY.into(),
                enabled: true,
                models: None,
                model_configs: None,
            },
            crypto.encrypt(PROVIDER_KEY).unwrap(),
        )
        .await
        .unwrap();
    db.upsert_upstream_models(
        upstream.id,
        &[UpstreamModelInput {
            model: "upstream-model".into(),
            enabled: true,
            context_window: 128_000,
            max_context_window: 128_000,
            supports_parallel_tool_calls: true,
            supports_reasoning_summaries: true,
            supports_image_input: false,
        }],
        None,
    )
    .await
    .unwrap();
    db.create_model_route(&ModelRouteInput {
        public_model: "public-model".into(),
        upstream_id: upstream.id,
        upstream_model: "upstream-model".into(),
        context_window: 128_000,
        max_context_window: 128_000,
        supports_parallel_tool_calls: true,
        supports_reasoning_summaries: true,
        supports_image_input: false,
        enabled: true,
    })
    .await
    .unwrap();
    db.create_api_key(
        &ApiKeyInput {
            name: "test-client".into(),
            enabled: true,
            models: None,
        },
        &crypto.hash_api_key(CLIENT_KEY),
        crypto.encrypt(CLIENT_KEY).unwrap(),
    )
    .await
    .unwrap();
}

#[derive(Clone, Default)]
struct UpstreamState {
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
}

#[derive(Clone, Debug)]
struct RecordedRequest {
    auth: Option<String>,
    body: Value,
}

async fn chat_json_upstream(
    State(state): State<UpstreamState>,
    req: axum::extract::Request,
) -> Response<Body> {
    let auth = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let bytes = to_bytes(req.into_body(), 50_000_000).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    state
        .requests
        .lock()
        .unwrap()
        .push(RecordedRequest { auth, body });

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "id": "chatcmpl_test",
                "object": "chat.completion",
                "model": "upstream-model",
                "choices": [{
                    "index": 0,
                    "message": {"role": "assistant", "content": "ok"},
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 1, "completion_tokens": 2, "total_tokens": 3}
            })
            .to_string(),
        ))
        .unwrap()
}

async fn chat_stream_upstream(
    State(state): State<UpstreamState>,
    req: axum::extract::Request,
) -> Response<Body> {
    let auth = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let bytes = to_bytes(req.into_body(), 50_000_000).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    state
        .requests
        .lock()
        .unwrap()
        .push(RecordedRequest { auth, body });

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/event-stream")
        .body(Body::from(concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"he\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"llo\"}}]}\n\n",
            "data: [DONE]\n\n"
        )))
        .unwrap()
}

async fn spawn_json_upstream() -> (String, UpstreamState) {
    let state = UpstreamState::default();
    let app = Router::new()
        .route(
            "/v1/chat/completions",
            axum::routing::post(chat_json_upstream),
        )
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    (format!("http://127.0.0.1:{port}/v1"), state)
}

async fn spawn_stream_upstream() -> (String, UpstreamState) {
    let state = UpstreamState::default();
    let app = Router::new()
        .route(
            "/v1/chat/completions",
            axum::routing::post(chat_stream_upstream),
        )
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    (format!("http://127.0.0.1:{port}/v1"), state)
}

#[tokio::test]
async fn chat_completions_rewrites_model_and_preserves_body() {
    let (upstream_base, upstream_state) = spawn_json_upstream().await;
    let relay = Relay::spawn(&upstream_base).await;

    let request = json!({
        "model": "public-model",
        "messages": [{"role": "user", "content": "hi"}],
        "stream": false,
        "temperature": 0.2,
        "reasoning": {"effort": "high"},
        "provider_extra": {"x": 1}
    });
    let response: Value = reqwest::Client::new()
        .post(relay.url("/v1/chat/completions"))
        .bearer_auth(CLIENT_KEY)
        .json(&request)
        .send()
        .await
        .expect("POST /v1/chat/completions")
        .error_for_status()
        .expect("chat status")
        .json()
        .await
        .expect("chat json");

    assert_eq!(response["object"], "chat.completion");
    assert_eq!(response["choices"][0]["message"]["content"], "ok");

    let recorded = upstream_state.requests.lock().unwrap().clone();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].auth.as_deref(), Some("Bearer provider-key"));
    assert_eq!(recorded[0].body["model"], "upstream-model");
    assert_eq!(recorded[0].body["messages"], request["messages"]);
    assert_eq!(recorded[0].body["reasoning"], request["reasoning"]);
    assert_eq!(
        recorded[0].body["provider_extra"],
        request["provider_extra"]
    );
}

#[tokio::test]
async fn chat_completions_stream_is_proxied_without_responses_events() {
    let (upstream_base, upstream_state) = spawn_stream_upstream().await;
    let relay = Relay::spawn(&upstream_base).await;

    let response = reqwest::Client::new()
        .post(relay.url("/v1/chat/completions"))
        .bearer_auth(CLIENT_KEY)
        .json(&json!({
            "model": "public-model",
            "messages": [{"role": "user", "content": "hi"}],
            "stream": true
        }))
        .send()
        .await
        .expect("POST /v1/chat/completions")
        .error_for_status()
        .expect("stream status");

    let mut events = response.bytes_stream().eventsource();
    let mut payloads = Vec::new();
    while let Some(event) = events.next().await {
        let event = event.expect("sse parse");
        payloads.push(event.data);
        if payloads.last().is_some_and(|data| data == "[DONE]") {
            break;
        }
    }

    assert_eq!(
        payloads[0],
        "{\"choices\":[{\"delta\":{\"content\":\"he\"}}]}"
    );
    assert_eq!(
        payloads[1],
        "{\"choices\":[{\"delta\":{\"content\":\"llo\"}}]}"
    );
    assert_eq!(payloads[2], "[DONE]");
    assert!(payloads.iter().all(|data| !data.contains("response.")));
    assert_eq!(
        upstream_state.requests.lock().unwrap()[0].body["model"],
        "upstream-model"
    );
}

#[tokio::test]
async fn chat_completions_unknown_model_returns_unknown_model() {
    let (upstream_base, upstream_state) = spawn_json_upstream().await;
    let relay = Relay::spawn(&upstream_base).await;

    let response = reqwest::Client::new()
        .post(relay.url("/v1/chat/completions"))
        .bearer_auth(CLIENT_KEY)
        .json(&json!({
            "model": "missing-model",
            "messages": [{"role": "user", "content": "hi"}]
        }))
        .send()
        .await
        .expect("POST /v1/chat/completions");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["code"], "UNKNOWN_MODEL");
    assert!(upstream_state.requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn chat_completions_missing_model_rejects_before_upstream() {
    let (upstream_base, upstream_state) = spawn_json_upstream().await;
    let relay = Relay::spawn(&upstream_base).await;

    let response = reqwest::Client::new()
        .post(relay.url("/v1/chat/completions"))
        .bearer_auth(CLIENT_KEY)
        .json(&json!({
            "messages": [{"role": "user", "content": "hi"}]
        }))
        .send()
        .await
        .expect("POST /v1/chat/completions");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["code"], "INVALID_REQUEST_BODY");
    assert!(upstream_state.requests.lock().unwrap().is_empty());
}
