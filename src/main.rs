mod db;
mod security;
mod session;
mod stream;
mod translate;
mod types;

use anyhow::{bail, Result};
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Path, Query, Request, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{Duration, Utc};
use clap::Parser;
use db::{ApiKeyInput, Db, LogInput, ModelRouteInput, UpstreamInput};
use reqwest::{Client, Url};
use security::{generate_api_key, generate_session_token, hash_password, verify_password, Crypto};
use serde::{Deserialize, Serialize};
use session::SessionStore;
use std::{sync::Arc, time::Instant};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing::{debug, error, info, warn};
use types::*;

#[derive(Parser, Debug)]
#[command(name = "chat2responses", about = "Responses API service proxy")]
struct Args {
    #[arg(long, env = "CHAT2RESPONSES_PORT", default_value = "4444")]
    port: u16,

    #[arg(
        long,
        env = "CHAT2RESPONSES_DATABASE_URL",
        default_value = "sqlite://data/chat2responses.db"
    )]
    database_url: String,

    #[arg(long, env = "CHAT2RESPONSES_SECRET", default_value = "")]
    secret: String,
}

#[derive(Clone)]
struct AppState {
    db: Db,
    sessions: SessionStore,
    client: Client,
    crypto: Arc<Crypto>,
}

#[derive(Deserialize)]
struct InitRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct AdminStatus {
    initialized: bool,
    user: Option<AdminUserView>,
}

#[derive(Serialize)]
struct AdminUserView {
    id: i64,
    username: String,
}

#[derive(Serialize)]
struct CreatedApiKey {
    id: i64,
    name: String,
    enabled: bool,
    created_at: String,
    key: String,
}

#[derive(Serialize)]
struct ApiErrorBody {
    code: &'static str,
    message: String,
}

#[derive(Serialize)]
struct UpstreamModels {
    data: Vec<serde_json::Value>,
    models: Vec<String>,
}

#[derive(Deserialize)]
struct LogsQuery {
    limit: Option<i64>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "chat2responses=info,tower_http=warn".into()),
        )
        .init();

    let args = Args::parse();
    let secret = if args.secret.trim().is_empty() {
        warn!("CHAT2RESPONSES_SECRET is not set; using an insecure development secret");
        "development-only-secret".to_string()
    } else {
        args.secret
    };

    let state = AppState {
        db: Db::connect(&args.database_url).await?,
        sessions: SessionStore::new(),
        client: Client::new(),
        crypto: Arc::new(Crypto::new(&secret)),
    };

    let app = build_router(state);
    let addr = format!("127.0.0.1:{}", args.port);
    info!("Chat2Responses service listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn build_router(state: AppState) -> Router {
    let api = Router::new()
        .route("/admin/api/status", get(admin_status))
        .route("/admin/api/init", post(admin_init))
        .route("/admin/api/login", post(admin_login))
        .route("/admin/api/logout", post(admin_logout))
        .route("/admin/api/upstreams", get(list_upstreams).post(create_upstream))
        .route(
            "/admin/api/upstreams/:id",
            put(update_upstream).delete(delete_upstream),
        )
        .route("/admin/api/upstreams/:id/models", get(fetch_upstream_models))
        .route("/admin/api/models", get(list_model_routes).post(create_model_route))
        .route(
            "/admin/api/models/:id",
            put(update_model_route).delete(delete_model_route),
        )
        .route("/admin/api/keys", get(list_api_keys).post(create_api_key))
        .route("/admin/api/keys/:id/enable", post(enable_api_key))
        .route("/admin/api/keys/:id/disable", post(disable_api_key))
        .route("/admin/api/keys/:id", delete(delete_api_key))
        .route("/admin/api/logs", get(list_logs));

    let static_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("admin-ui")
        .join("dist");

    Router::new()
        .merge(api)
        .route("/v1/responses", post(handle_responses))
        .route("/v1/models", get(handle_models))
        .route("/", get(|| async { Redirect::temporary("/admin/") }))
        .nest_service("/assets", ServeDir::new(static_dir.join("assets")))
        .route("/admin/", get(admin_index))
        .fallback(handle_fallback)
        .layer(DefaultBodyLimit::disable())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn admin_index() -> Response {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("admin-ui")
        .join("dist")
        .join("index.html");
    match tokio::fs::read_to_string(path).await {
        Ok(html) => Html(html).into_response(),
        Err(_) => Html(
            r#"<html><body><h1>Admin UI is not built</h1><p>Run <code>npm install && npm run build</code> in <code>admin-ui</code>.</p></body></html>"#,
        )
        .into_response(),
    }
}

async fn admin_status(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let initialized = match state.db.has_admin().await {
        Ok(v) => v,
        Err(e) => return internal_error(e),
    };
    let user = match current_admin(&state, &headers).await {
        Ok(Some(user)) => Some(AdminUserView {
            id: user.id,
            username: user.username,
        }),
        Ok(None) => None,
        Err(e) => return internal_error(e),
    };
    Json(AdminStatus { initialized, user }).into_response()
}

async fn admin_init(State(state): State<AppState>, Json(req): Json<InitRequest>) -> Response {
    match state.db.has_admin().await {
        Ok(true) => return api_error(StatusCode::CONFLICT, "ADMIN_ALREADY_INITIALIZED", "Admin already initialized"),
        Ok(false) => {}
        Err(e) => return internal_error(e),
    }
    if req.username.trim().is_empty() || req.password.len() < 8 {
        return api_error(
            StatusCode::BAD_REQUEST,
            "BAD_REQUEST",
            "Username required and password must be at least 8 characters",
        );
    }
    let hash = match hash_password(&req.password) {
        Ok(h) => h,
        Err(e) => return internal_error(e),
    };
    match state.db.create_admin(req.username.trim(), &hash).await {
        Ok(user) => create_admin_session_response(&state, user.id, AdminUserView {
            id: user.id,
            username: user.username,
        })
        .await,
        Err(e) => internal_error(e),
    }
}

async fn admin_login(State(state): State<AppState>, Json(req): Json<LoginRequest>) -> Response {
    let user = match state.db.find_admin_by_username(req.username.trim()).await {
        Ok(Some(user)) => user,
        Ok(None) => return api_error(StatusCode::UNAUTHORIZED, "INVALID_CREDENTIALS", "Invalid credentials"),
        Err(e) => return internal_error(e),
    };
    if !verify_password(&req.password, &user.password_hash) {
        return api_error(StatusCode::UNAUTHORIZED, "INVALID_CREDENTIALS", "Invalid credentials");
    }
    create_admin_session_response(&state, user.id, AdminUserView {
        id: user.id,
        username: user.username,
    })
    .await
}

async fn admin_logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(token) = session_cookie(&headers) {
        let hash = state.crypto.hash_api_key(&token);
        if let Err(e) = state.db.delete_admin_session(&hash).await {
            return internal_error(e);
        }
    }
    let mut resp = StatusCode::NO_CONTENT.into_response();
    clear_session_cookie(resp.headers_mut());
    resp
}

async fn list_upstreams(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    match state.db.list_upstreams().await {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => internal_error(e),
    }
}

async fn create_upstream(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<UpstreamInput>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    if let Err(resp) = validate_upstream_input(&input) {
        return resp;
    }
    let encrypted = match state.crypto.encrypt(&input.api_key) {
        Ok(v) => v,
        Err(e) => return internal_error(e),
    };
    match state.db.create_upstream(&input, encrypted).await {
        Ok(row) => Json(row).into_response(),
        Err(e) => internal_error(e),
    }
}

async fn update_upstream(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(input): Json<UpstreamInput>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    if let Err(resp) = validate_upstream_input(&input) {
        return resp;
    }
    let encrypted = if input.api_key.trim().is_empty() {
        None
    } else {
        match state.crypto.encrypt(&input.api_key) {
            Ok(v) => Some(v),
            Err(e) => return internal_error(e),
        }
    };
    match state.db.update_upstream(id, &input, encrypted).await {
        Ok(Some(row)) => Json(row).into_response(),
        Ok(None) => api_error(StatusCode::NOT_FOUND, "NOT_FOUND", "Upstream not found"),
        Err(e) => internal_error(e),
    }
}

async fn delete_upstream(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    match state.db.delete_upstream(id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => internal_error(e),
    }
}

async fn fetch_upstream_models(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    let Some(upstream) = (match state.db.get_upstream(id).await {
        Ok(v) => v,
        Err(e) => return internal_error(e),
    }) else {
        return api_error(StatusCode::NOT_FOUND, "NOT_FOUND", "Upstream not found");
    };
    let api_key = match state.crypto.decrypt(&upstream.encrypted_api_key) {
        Ok(v) => v,
        Err(e) => return internal_error(e),
    };
    let url = format!("{}models", join_base_str(&upstream.base_url));
    let mut builder = state.client.get(url);
    if !api_key.is_empty() {
        builder = builder.bearer_auth(api_key);
    }
    match builder.send().await {
        Ok(r) if r.status().is_success() => match r.json::<serde_json::Value>().await {
            Ok(body) => {
                let data = model_list_from_body(&body);
                let models = data
                    .iter()
                    .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
                    .collect();
                Json(UpstreamModels { data, models }).into_response()
            }
            Err(e) => internal_error(e),
        },
        Ok(r) => {
            let status = r.status();
            let body = r.text().await.unwrap_or_default();
            (
                StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
                body,
            )
                .into_response()
        }
        Err(e) => api_error(StatusCode::BAD_GATEWAY, "UPSTREAM_CONNECTION_ERROR", e.to_string()),
    }
}

async fn list_model_routes(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    match state.db.list_model_routes(false).await {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => internal_error(e),
    }
}

async fn create_model_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ModelRouteInput>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    if input.public_model.trim().is_empty() || input.upstream_model.trim().is_empty() {
        return api_error(
            StatusCode::BAD_REQUEST,
            "BAD_REQUEST",
            "public_model and upstream_model are required",
        );
    }
    match state.db.create_model_route(&input).await {
        Ok(row) => Json(row).into_response(),
        Err(e) => internal_error(e),
    }
}

async fn update_model_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(input): Json<ModelRouteInput>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    match state.db.update_model_route(id, &input).await {
        Ok(Some(row)) => Json(row).into_response(),
        Ok(None) => api_error(StatusCode::NOT_FOUND, "NOT_FOUND", "Model route not found"),
        Err(e) => internal_error(e),
    }
}

async fn delete_model_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    match state.db.delete_model_route(id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => internal_error(e),
    }
}

async fn list_api_keys(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    match state.db.list_api_keys().await {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => internal_error(e),
    }
}

async fn create_api_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ApiKeyInput>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    if input.name.trim().is_empty() {
        return api_error(StatusCode::BAD_REQUEST, "BAD_REQUEST", "name is required");
    }
    let key = generate_api_key();
    let key_hash = state.crypto.hash_api_key(&key);
    match state.db.create_api_key(&input, &key_hash).await {
        Ok(row) => Json(CreatedApiKey {
            id: row.id,
            name: row.name,
            enabled: row.enabled,
            created_at: row.created_at,
            key,
        })
        .into_response(),
        Err(e) => internal_error(e),
    }
}

async fn enable_api_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    set_api_key_enabled(state, headers, id, true).await
}

async fn disable_api_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    set_api_key_enabled(state, headers, id, false).await
}

async fn set_api_key_enabled(
    state: AppState,
    headers: HeaderMap,
    id: i64,
    enabled: bool,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    match state.db.set_api_key_enabled(id, enabled).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => internal_error(e),
    }
}

async fn delete_api_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    match state.db.delete_api_key(id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => internal_error(e),
    }
}

async fn list_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<LogsQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    match state.db.list_request_logs(query.limit.unwrap_or(100)).await {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => internal_error(e),
    }
}

async fn handle_models(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let auth = match authenticate_api_key(&state, &headers).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let _ = state.db.mark_api_key_used(auth.id).await;
    match state.db.list_model_routes(true).await {
        Ok(routes) => {
            let data: Vec<_> = routes
                .iter()
                .map(|route| {
                    serde_json::json!({
                        "id": route.public_model,
                        "object": "model",
                        "owned_by": route.upstream_name,
                    })
                })
                .collect();
            Json(serde_json::json!({
                "object": "list",
                "data": data.clone(),
                "models": data,
            }))
            .into_response()
        }
        Err(e) => internal_error(e),
    }
}

async fn handle_responses(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let started = Instant::now();
    let auth = match authenticate_api_key(&state, &headers).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let _ = state.db.mark_api_key_used(auth.id).await;

    let req: ResponsesRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            error!("JSON parse error: {e}");
            return api_error(StatusCode::UNPROCESSABLE_ENTITY, "INVALID_REQUEST_BODY", e.to_string());
        }
    };

    debug!(
        "response request key={} model={} stream={}",
        auth.name, req.model, req.stream
    );

    let Some(route) = (match state.db.find_model_route(&req.model).await {
        Ok(v) => v,
        Err(e) => return internal_error(e),
    }) else {
        let _ = state
            .db
            .insert_request_log(LogInput {
                api_key_id: Some(auth.id),
                public_model: Some(req.model.clone()),
                upstream_id: None,
                upstream_model: None,
                status_code: StatusCode::BAD_REQUEST.as_u16(),
                input_tokens: 0,
                output_tokens: 0,
                total_tokens: 0,
                error: Some("unknown or disabled model".into()),
                duration_ms: started.elapsed().as_millis() as i64,
            })
            .await;
        return api_error(StatusCode::BAD_REQUEST, "UNKNOWN_MODEL", "Unknown or disabled model");
    };

    let Some(upstream) = (match state.db.get_upstream(route.upstream_id).await {
        Ok(v) => v,
        Err(e) => return internal_error(e),
    }) else {
        return api_error(StatusCode::BAD_GATEWAY, "UPSTREAM_NOT_FOUND", "Upstream not found");
    };
    if !upstream.enabled {
        return api_error(StatusCode::BAD_GATEWAY, "UPSTREAM_DISABLED", "Upstream disabled");
    }

    let upstream_key = match state.crypto.decrypt(&upstream.encrypted_api_key) {
        Ok(v) => Arc::new(v),
        Err(e) => return internal_error(e),
    };
    let upstream_url = format!("{}chat/completions", join_base_str(&upstream.base_url));
    handle_responses_inner(
        state,
        req,
        ProxyTarget {
            api_key_id: auth.id,
            public_model: route.public_model,
            upstream_id: upstream.id,
            upstream_model: route.upstream_model,
            upstream_url,
            upstream_api_key: upstream_key,
            started,
        },
    )
    .await
}

struct ProxyTarget {
    api_key_id: i64,
    public_model: String,
    upstream_id: i64,
    upstream_model: String,
    upstream_url: String,
    upstream_api_key: Arc<String>,
    started: Instant,
}

async fn handle_responses_inner(state: AppState, req: ResponsesRequest, target: ProxyTarget) -> Response {
    let history = req
        .previous_response_id
        .as_deref()
        .map(|id| state.sessions.get_history(id))
        .unwrap_or_default();

    let mut chat_req = translate::to_chat_request_with_model(
        &req,
        history.clone(),
        &state.sessions,
        target.upstream_model.clone(),
    );

    if req.stream {
        let response_id = state.sessions.new_id();
        chat_req.stream = true;
        let request_messages = chat_req.messages.clone();
        stream::translate_stream(stream::StreamArgs {
            client: state.client,
            url: target.upstream_url.clone(),
            api_key: target.upstream_api_key.clone(),
            chat_req,
            response_id,
            sessions: state.sessions,
            request_messages,
            model: target.public_model.clone(),
            on_complete: Some(stream_log_callback(state.db.clone(), target)),
        })
        .into_response()
    } else {
        chat_req.stream = false;
        handle_blocking(state, chat_req, target).await
    }
}

fn stream_log_callback(
    db: Db,
    target: ProxyTarget,
) -> Arc<dyn Fn(stream::StreamLog) + Send + Sync> {
    Arc::new(move |entry| {
        let db = db.clone();
        let input = LogInput {
            api_key_id: Some(target.api_key_id),
            public_model: Some(target.public_model.clone()),
            upstream_id: Some(target.upstream_id),
            upstream_model: Some(target.upstream_model.clone()),
            status_code: entry.status_code,
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            error: entry.error.map(|e| e.chars().take(500).collect()),
            duration_ms: target.started.elapsed().as_millis() as i64,
        };
        tokio::spawn(async move {
            let _ = db.insert_request_log(input).await;
        });
    })
}

async fn handle_blocking(state: AppState, chat_req: types::ChatRequest, target: ProxyTarget) -> Response {
    let mut builder = state
        .client
        .post(&target.upstream_url)
        .header("Content-Type", "application/json");

    if !target.upstream_api_key.is_empty() {
        builder = builder.bearer_auth(target.upstream_api_key.as_str());
    }

    match builder.json(&chat_req).send().await {
        Err(e) => {
            error!("upstream error: {e}");
            log_request(&state, &target, StatusCode::BAD_GATEWAY, None, Some(e.to_string())).await;
            api_error(StatusCode::BAD_GATEWAY, "UPSTREAM_CONNECTION_ERROR", e.to_string())
        }
        Ok(r) if !r.status().is_success() => {
            let status = r.status();
            let body = r.text().await.unwrap_or_default();
            error!("upstream {status}: {body}");
            log_request(&state, &target, status, None, Some(body.clone())).await;
            (
                StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
                body,
            )
                .into_response()
        }
        Ok(r) => match r.json::<ChatResponse>().await {
            Err(e) => {
                error!("parse error: {e}");
                log_request(&state, &target, StatusCode::INTERNAL_SERVER_ERROR, None, Some(e.to_string())).await;
                api_error(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", e.to_string())
            }
            Ok(chat_resp) => {
                let usage = chat_resp.usage.as_ref().map(|u| {
                    (
                        i64::from(u.prompt_tokens),
                        i64::from(u.completion_tokens),
                        i64::from(u.total_tokens),
                    )
                });
                let assistant_msg = chat_resp
                    .choices
                    .first()
                    .map(|c| c.message.clone())
                    .unwrap_or_else(|| ChatMessage {
                        role: "assistant".into(),
                        content: Some(serde_json::Value::String(String::new())),
                        reasoning_content: None,
                        tool_calls: None,
                        tool_call_id: None,
                        name: None,
                    });

                let mut full_history = chat_req.messages.clone();
                full_history.push(assistant_msg);
                let response_id = state.sessions.save(full_history);

                let (resp, _) = translate::from_chat_response(response_id, &target.public_model, chat_resp);
                log_request(&state, &target, StatusCode::OK, usage, None).await;
                Json(resp).into_response()
            }
        },
    }
}

async fn log_request(
    state: &AppState,
    target: &ProxyTarget,
    status: StatusCode,
    usage: Option<(i64, i64, i64)>,
    error: Option<String>,
) {
    let (input_tokens, output_tokens, total_tokens) = usage.unwrap_or((0, 0, 0));
    let error = error.map(|e| e.chars().take(500).collect());
    let _ = state
        .db
        .insert_request_log(LogInput {
            api_key_id: Some(target.api_key_id),
            public_model: Some(target.public_model.clone()),
            upstream_id: Some(target.upstream_id),
            upstream_model: Some(target.upstream_model.clone()),
            status_code: status.as_u16(),
            input_tokens,
            output_tokens,
            total_tokens,
            error,
            duration_ms: target.started.elapsed().as_millis() as i64,
        })
        .await;
}

async fn authenticate_api_key(
    state: &AppState,
    headers: &HeaderMap,
) -> std::result::Result<db::ApiKeyRecord, Response> {
    let Some(raw) = bearer_token(headers) else {
        return Err(api_error(StatusCode::UNAUTHORIZED, "MISSING_API_KEY", "Missing API key"));
    };
    let hash = state.crypto.hash_api_key(&raw);
    match state.db.find_api_key_by_hash(&hash).await {
        Ok(Some(record)) if record.enabled => Ok(record),
        Ok(Some(_)) => Err(api_error(StatusCode::UNAUTHORIZED, "DISABLED_API_KEY", "Disabled API key")),
        Ok(None) => Err(api_error(StatusCode::UNAUTHORIZED, "INVALID_API_KEY", "Invalid API key")),
        Err(e) => Err(internal_error(e)),
    }
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

async fn current_admin(state: &AppState, headers: &HeaderMap) -> Result<Option<db::AdminUser>> {
    let Some(token) = session_cookie(headers) else {
        return Ok(None);
    };
    let hash = state.crypto.hash_api_key(&token);
    state.db.admin_for_session(&hash).await
}

async fn require_admin(state: &AppState, headers: &HeaderMap) -> std::result::Result<db::AdminUser, Response> {
    match current_admin(state, headers).await {
        Ok(Some(user)) => Ok(user),
        Ok(None) => Err(api_error(
            StatusCode::UNAUTHORIZED,
            "ADMIN_LOGIN_REQUIRED",
            "Admin login required",
        )),
        Err(e) => Err(internal_error(e)),
    }
}

async fn create_admin_session_response(
    state: &AppState,
    admin_id: i64,
    user: AdminUserView,
) -> Response {
    let token = generate_session_token();
    let token_hash = state.crypto.hash_api_key(&token);
    let expires_at = (Utc::now() + Duration::days(14)).to_rfc3339();
    if let Err(e) = state
        .db
        .create_admin_session(&token_hash, admin_id, &expires_at)
        .await
    {
        return internal_error(e);
    }
    let mut resp = Json(user).into_response();
    set_session_cookie(resp.headers_mut(), &token);
    resp
}

fn session_cookie(headers: &HeaderMap) -> Option<String> {
    let cookie = headers.get(header::COOKIE)?.to_str().ok()?;
    cookie.split(';').find_map(|part| {
        let part = part.trim();
        let (name, value) = part.split_once('=')?;
        (name == "chat2responses_admin").then(|| value.to_string())
    })
}

fn set_session_cookie(headers: &mut HeaderMap, token: &str) {
    let value = format!(
        "chat2responses_admin={token}; HttpOnly; SameSite=Lax; Path=/; Max-Age={}",
        14 * 24 * 60 * 60
    );
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&value).expect("valid cookie header"),
    );
}

fn clear_session_cookie(headers: &mut HeaderMap) {
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_static("chat2responses_admin=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0"),
    );
}

fn validate_upstream_input(input: &UpstreamInput) -> std::result::Result<(), Response> {
    if input.name.trim().is_empty() || input.base_url.trim().is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "BAD_REQUEST",
            "name and base_url are required",
        ));
    }
    validate_upstream(input.base_url.trim())
        .map(|_| ())
        .map_err(|e| api_error(StatusCode::BAD_REQUEST, "BAD_REQUEST", e.to_string()))
}

fn validate_upstream(raw: &str) -> Result<Url> {
    let url = Url::parse(raw.trim_end_matches('/'))?;
    match url.scheme() {
        "http" | "https" => {}
        s => bail!("upstream URL scheme must be http or https, got: {s}"),
    }
    if url.host_str().is_none() {
        bail!("upstream URL must have a host");
    }
    Ok(url)
}

fn join_base_str(url: &str) -> String {
    if url.ends_with('/') {
        url.to_string()
    } else {
        format!("{url}/")
    }
}

fn model_list_from_body(body: &serde_json::Value) -> Vec<serde_json::Value> {
    body.get("data")
        .or_else(|| body.get("models"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

async fn handle_fallback(req: Request<Body>) -> Response {
    if req.method() == Method::GET && req.uri().path().starts_with("/admin/") {
        return admin_index().await;
    }
    warn!("unhandled {} {}", req.method(), req.uri().path());
    api_error(StatusCode::NOT_FOUND, "NOT_FOUND", "Not found")
}

fn api_error(status: StatusCode, code: &'static str, message: impl Into<String>) -> Response {
    (status, Json(ApiErrorBody { code, message: message.into() })).into_response()
}

fn internal_error(err: impl std::fmt::Display) -> Response {
    error!("{err}");
    api_error(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_upstream_https() {
        let url = validate_upstream("https://openrouter.ai/api/v1").unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("openrouter.ai"));
    }

    #[test]
    fn test_validate_upstream_rejects_ftp() {
        assert!(validate_upstream("ftp://evil.com").is_err());
    }

    #[test]
    fn test_join_base_adds_trailing_slash() {
        assert_eq!(join_base_str("https://api.example.com/v1"), "https://api.example.com/v1/");
    }

    #[test]
    fn test_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer cr_test"));
        assert_eq!(bearer_token(&headers).as_deref(), Some("cr_test"));
    }

    #[test]
    fn test_model_list_from_body_supports_data_and_models() {
        let data = serde_json::json!({"data":[{"id":"a"}]});
        let models = serde_json::json!({"models":[{"id":"b"}]});
        assert_eq!(model_list_from_body(&data)[0]["id"], "a");
        assert_eq!(model_list_from_body(&models)[0]["id"], "b");
    }

    #[tokio::test]
    async fn test_api_error_shape() {
        let resp = api_error(StatusCode::UNAUTHORIZED, "INVALID_API_KEY", "Invalid API key");
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json, serde_json::json!({
            "code": "INVALID_API_KEY",
            "message": "Invalid API key"
        }));
    }
}
