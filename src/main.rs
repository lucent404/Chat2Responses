mod codex_catalog;
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
use db::{
    ApiKeyInput, AppSettings, Db, LogInput, ModelRouteInput, PageParams, UpstreamInput,
    UpstreamModelInput,
};
use reqwest::{Client, Url};
use security::{generate_api_key, generate_session_token, hash_password, verify_password, Crypto};
use serde::{Deserialize, Serialize};
use session::SessionStore;
use std::{sync::Arc, time::Duration as StdDuration, time::Instant};
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
    masked_key: String,
    models: Vec<String>,
    key: String,
}

#[derive(Serialize)]
struct ApiKeyView {
    id: i64,
    name: String,
    enabled: bool,
    created_at: String,
    last_used_at: Option<String>,
    masked_key: Option<String>,
    key_recoverable: bool,
    models: Vec<String>,
}

#[derive(Serialize)]
struct RevealedApiKey {
    key: String,
    masked_key: String,
}

#[derive(Serialize)]
struct Paginated<T> {
    items: Vec<T>,
    total: i64,
    page: i64,
    page_size: i64,
    total_pages: i64,
}

#[derive(Deserialize)]
struct DiscoverModelsInput {
    base_url: String,
    #[serde(default)]
    api_key: String,
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
    model_configs: Vec<UpstreamModelInput>,
}

#[derive(Deserialize)]
struct SaveUpstreamModelsRequest {
    models: Vec<UpstreamModelInput>,
}

#[derive(Deserialize)]
struct PageQuery {
    page: Option<i64>,
    page_size: Option<i64>,
    q: Option<String>,
}

#[derive(Deserialize)]
struct SettingsInput {
    request_logging_enabled: bool,
    upstream_timeout_seconds: i64,
    log_error_max_chars: i64,
}

const CODEX_BASE_INSTRUCTIONS: &str = "You are Codex, a coding agent. You help the user work in their local development environment, inspect the repository before making assumptions, make focused code changes when requested, and communicate clearly about what changed and how it was verified.";
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
        .route(
            "/admin/api/upstreams",
            get(list_upstreams).post(create_upstream),
        )
        .route(
            "/admin/api/upstreams/discover-models",
            post(discover_upstream_models),
        )
        .route(
            "/admin/api/upstreams/:id",
            put(update_upstream).delete(delete_upstream),
        )
        .route(
            "/admin/api/upstreams/:id/models",
            get(fetch_upstream_models).put(save_local_upstream_models),
        )
        .route(
            "/admin/api/upstreams/:id/models/local",
            get(list_local_upstream_models),
        )
        .route("/admin/api/available-models", get(list_available_models))
        .route(
            "/admin/api/models",
            get(list_model_routes).post(create_model_route),
        )
        .route(
            "/admin/api/models/:id",
            put(update_model_route).delete(delete_model_route),
        )
        .route("/admin/api/keys", get(list_api_keys).post(create_api_key))
        .route("/admin/api/keys/:id/reveal", get(reveal_api_key))
        .route("/admin/api/keys/:id/enable", post(enable_api_key))
        .route("/admin/api/keys/:id/disable", post(disable_api_key))
        .route("/admin/api/keys/:id", delete(delete_api_key))
        .route("/admin/api/logs", get(list_logs))
        .route("/admin/api/codex-catalog/status", get(codex_catalog_status))
        .route(
            "/admin/api/codex-catalog/download",
            get(download_codex_catalog),
        )
        .route(
            "/admin/api/settings",
            get(get_settings).put(update_settings),
        );

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
        Ok(true) => {
            return api_error(
                StatusCode::CONFLICT,
                "ADMIN_ALREADY_INITIALIZED",
                "Admin already initialized",
            )
        }
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
        Ok(user) => {
            create_admin_session_response(
                &state,
                user.id,
                AdminUserView {
                    id: user.id,
                    username: user.username,
                },
            )
            .await
        }
        Err(e) => internal_error(e),
    }
}

async fn admin_login(State(state): State<AppState>, Json(req): Json<LoginRequest>) -> Response {
    let user = match state.db.find_admin_by_username(req.username.trim()).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return api_error(
                StatusCode::UNAUTHORIZED,
                "INVALID_CREDENTIALS",
                "Invalid credentials",
            )
        }
        Err(e) => return internal_error(e),
    };
    if !verify_password(&req.password, &user.password_hash) {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "INVALID_CREDENTIALS",
            "Invalid credentials",
        );
    }
    create_admin_session_response(
        &state,
        user.id,
        AdminUserView {
            id: user.id,
            username: user.username,
        },
    )
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

async fn list_upstreams(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PageQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    let params = page_params(query);
    match state.db.list_upstreams_paged(&params).await {
        Ok((rows, total)) => Json(paginated(rows, total, &params)).into_response(),
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
    let mut discovered =
        match fetch_models_from_upstream(&state.client, &input.base_url, &input.api_key).await {
            Ok(models) => models.model_configs,
            Err(resp) => return resp,
        };
    if let Some(configs) = input.model_configs.as_deref() {
        merge_model_configs(&mut discovered, configs);
    }
    let encrypted = match state.crypto.encrypt(&input.api_key) {
        Ok(v) => v,
        Err(e) => return internal_error(e),
    };
    match state.db.create_upstream(&input, encrypted).await {
        Ok(row) => {
            if let Err(e) = state
                .db
                .upsert_upstream_models(row.id, &discovered, input.models.as_deref())
                .await
            {
                return internal_error(e);
            }
            Json(row).into_response()
        }
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
        Ok(Some(row)) => {
            if let Err(e) = state
                .db
                .set_upstream_enabled_models(row.id, input.models.as_deref())
                .await
            {
                return internal_error(e);
            }
            Json(row).into_response()
        }
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
    match fetch_models_from_upstream(&state.client, &upstream.base_url, &api_key).await {
        Ok(models) => {
            if let Err(e) = state
                .db
                .sync_upstream_model_inventory(upstream.id, &models.model_configs)
                .await
            {
                return internal_error(e);
            }
            Json(models).into_response()
        }
        Err(resp) => resp,
    }
}

async fn list_local_upstream_models(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    match state.db.list_local_upstream_models(id).await {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => internal_error(e),
    }
}

async fn save_local_upstream_models(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(input): Json<SaveUpstreamModelsRequest>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    match state.db.save_upstream_models(id, &input.models).await {
        Ok(()) => match state.db.list_local_upstream_models(id).await {
            Ok(rows) => Json(rows).into_response(),
            Err(e) => internal_error(e),
        },
        Err(e) => internal_error(e),
    }
}

async fn discover_upstream_models(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<DiscoverModelsInput>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    if let Err(e) = validate_upstream(input.base_url.trim()) {
        return api_error(StatusCode::BAD_REQUEST, "BAD_REQUEST", e.to_string());
    }
    match fetch_models_from_upstream(&state.client, &input.base_url, &input.api_key).await {
        Ok(models) => Json(models).into_response(),
        Err(resp) => resp,
    }
}

async fn list_model_routes(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PageQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    let params = page_params(query);
    match state.db.list_model_routes_paged(&params).await {
        Ok((rows, total)) => Json(paginated(rows, total, &params)).into_response(),
        Err(e) => internal_error(e),
    }
}

async fn list_available_models(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    match state.db.list_available_models_for_key(None).await {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => internal_error(e),
    }
}

async fn codex_catalog_status(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    match state.db.list_available_models_for_key(None).await {
        Ok(models) => Json(codex_catalog::catalog_status(models.len())).into_response(),
        Err(e) => internal_error(e),
    }
}

async fn download_codex_catalog(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    let models = match state.db.list_available_models_for_key(None).await {
        Ok(models) => models,
        Err(e) => return internal_error(e),
    };
    match codex_catalog::generate_catalog_json(&models) {
        Ok(catalog) => (
            [
                (
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json; charset=utf-8"),
                ),
                (
                    header::CONTENT_DISPOSITION,
                    HeaderValue::from_static("attachment; filename=\"model-catalog.json\""),
                ),
            ],
            catalog,
        )
            .into_response(),
        Err(e) => api_error(
            StatusCode::BAD_REQUEST,
            "CATALOG_GENERATION_FAILED",
            e.to_string(),
        ),
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

async fn list_api_keys(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PageQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    let params = page_params(query);
    match state.db.list_api_keys_paged(&params).await {
        Ok((rows, total)) => {
            let rows: Vec<_> = rows
                .into_iter()
                .map(|(key, models)| api_key_view(&state, key, models))
                .collect();
            Json(paginated(rows, total, &params)).into_response()
        }
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
    let encrypted_key = match state.crypto.encrypt(&key) {
        Ok(value) => value,
        Err(e) => return internal_error(e),
    };
    match state
        .db
        .create_api_key(&input, &key_hash, encrypted_key)
        .await
    {
        Ok(row) => Json(CreatedApiKey {
            id: row.id,
            name: row.name,
            enabled: row.enabled,
            created_at: row.created_at,
            masked_key: mask_api_key(&key),
            models: input.models.unwrap_or_default(),
            key,
        })
        .into_response(),
        Err(e) => internal_error(e),
    }
}

async fn reveal_api_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    let Some(row) = (match state.db.get_api_key(id).await {
        Ok(row) => row,
        Err(e) => return internal_error(e),
    }) else {
        return api_error(StatusCode::NOT_FOUND, "NOT_FOUND", "API key not found");
    };
    let Some(encrypted_key) = row.encrypted_key else {
        return api_error(
            StatusCode::NOT_FOUND,
            "KEY_NOT_RECOVERABLE",
            "This API key was created before encrypted key storage was enabled",
        );
    };
    match state.crypto.decrypt(&encrypted_key) {
        Ok(key) => Json(RevealedApiKey {
            masked_key: mask_api_key(&key),
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
    Query(query): Query<PageQuery>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    let params = page_params(query);
    match state.db.list_request_logs_paged(&params).await {
        Ok((rows, total)) => Json(paginated(rows, total, &params)).into_response(),
        Err(e) => internal_error(e),
    }
}

async fn get_settings(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    match state.db.get_app_settings().await {
        Ok(settings) => Json(settings).into_response(),
        Err(e) => internal_error(e),
    }
}

async fn update_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<SettingsInput>,
) -> Response {
    if let Err(resp) = require_admin(&state, &headers).await {
        return resp;
    }
    let settings = AppSettings {
        request_logging_enabled: input.request_logging_enabled,
        upstream_timeout_seconds: input.upstream_timeout_seconds,
        log_error_max_chars: input.log_error_max_chars,
    };
    if let Err(resp) = validate_settings(&settings) {
        return resp;
    }
    match state.db.save_app_settings(&settings).await {
        Ok(settings) => Json(settings).into_response(),
        Err(e) => internal_error(e),
    }
}

async fn handle_models(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let auth = match authenticate_api_key(&state, &headers).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let _ = state.db.mark_api_key_used(auth.id).await;
    match state.db.list_available_models_for_key(Some(auth.id)).await {
        Ok(models) => {
            let data: Vec<_> = models
                .iter()
                .map(|model| {
                    serde_json::json!({
                        "id": model.id,
                        "object": "model",
                        "owned_by": model.owner,
                    })
                })
                .collect();
            Json(serde_json::json!({
                "object": "list",
                "data": data.clone(),
                "models": models.iter().enumerate().map(|(index, model)| codex_model_metadata(model, index)).collect::<Vec<_>>(),
            }))
            .into_response()
        }
        Err(e) => internal_error(e),
    }
}

fn codex_model_metadata(model: &db::AvailableModel, index: usize) -> serde_json::Value {
    let context_window = model.context_window.max(1);
    let max_context_window = model.max_context_window.max(context_window);
    serde_json::json!({
        "slug": model.id,
        "display_name": model.id,
        "description": format!("{} via Chat2Responses", model.owner),
        "default_reasoning_level": null,
        "supported_reasoning_levels": [],
        "shell_type": "shell_command",
        "visibility": "list",
        "supported_in_api": true,
        "priority": 100 + index as i64,
        "additional_speed_tiers": [],
        "service_tiers": [],
        "availability_nux": null,
        "upgrade": null,
        "base_instructions": CODEX_BASE_INSTRUCTIONS,
        "model_messages": null,
        "supports_reasoning_summaries": model.supports_reasoning_summaries,
        "default_reasoning_summary": "auto",
        "support_verbosity": false,
        "default_verbosity": null,
        "apply_patch_tool_type": null,
        "web_search_tool_type": "text",
        "truncation_policy": {
            "mode": "tokens",
            "limit": 10000,
        },
        "supports_parallel_tool_calls": model.supports_parallel_tool_calls,
        "supports_image_detail_original": false,
        "context_window": context_window,
        "max_context_window": max_context_window,
        "auto_compact_token_limit": null,
        "effective_context_window_percent": 95,
        "experimental_supported_tools": [],
        "input_modalities": ["text"],
        "supports_search_tool": false,
    })
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
            return api_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_REQUEST_BODY",
                e.to_string(),
            );
        }
    };

    debug!(
        "response request key={} model={} stream={}",
        auth.name, req.model, req.stream
    );

    let candidates = match state.db.list_route_candidates(auth.id, &req.model).await {
        Ok(v) => v,
        Err(e) => return internal_error(e),
    };
    let Some(route) = choose_route_candidate(candidates) else {
        write_request_log(
            &state.db,
            LogInput {
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
            },
        )
        .await;
        return api_error(
            StatusCode::BAD_REQUEST,
            "UNKNOWN_MODEL",
            "Unknown or disabled model",
        );
    };

    let Some(upstream) = (match state.db.get_upstream(route.upstream_id).await {
        Ok(v) => v,
        Err(e) => return internal_error(e),
    }) else {
        return api_error(
            StatusCode::BAD_GATEWAY,
            "UPSTREAM_NOT_FOUND",
            "Upstream not found",
        );
    };
    if !upstream.enabled {
        return api_error(
            StatusCode::BAD_GATEWAY,
            "UPSTREAM_DISABLED",
            "Upstream disabled",
        );
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

async fn handle_responses_inner(
    state: AppState,
    req: ResponsesRequest,
    target: ProxyTarget,
) -> Response {
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
        let settings = match state.db.get_app_settings().await {
            Ok(settings) => settings,
            Err(e) => return internal_error(e),
        };
        stream::translate_stream(stream::StreamArgs {
            client: state.client,
            url: target.upstream_url.clone(),
            api_key: target.upstream_api_key.clone(),
            chat_req,
            response_id,
            sessions: state.sessions,
            request_messages,
            model: target.public_model.clone(),
            upstream_timeout_seconds: settings.upstream_timeout_seconds,
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
            error: entry.error,
            duration_ms: target.started.elapsed().as_millis() as i64,
        };
        tokio::spawn(async move {
            write_request_log(&db, input).await;
        });
    })
}

async fn handle_blocking(
    state: AppState,
    chat_req: types::ChatRequest,
    target: ProxyTarget,
) -> Response {
    let mut builder = state
        .client
        .post(&target.upstream_url)
        .header("Content-Type", "application/json");

    if !target.upstream_api_key.is_empty() {
        builder = builder.bearer_auth(target.upstream_api_key.as_str());
    }

    let settings = match state.db.get_app_settings().await {
        Ok(settings) => settings,
        Err(e) => return internal_error(e),
    };
    if settings.upstream_timeout_seconds > 0 {
        builder = builder.timeout(StdDuration::from_secs(
            settings.upstream_timeout_seconds as u64,
        ));
    }

    match builder.json(&chat_req).send().await {
        Err(e) => {
            error!("upstream error: {e}");
            log_request(
                &state,
                &target,
                StatusCode::BAD_GATEWAY,
                None,
                Some(e.to_string()),
            )
            .await;
            api_error(
                StatusCode::BAD_GATEWAY,
                "UPSTREAM_CONNECTION_ERROR",
                e.to_string(),
            )
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
                log_request(
                    &state,
                    &target,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    None,
                    Some(e.to_string()),
                )
                .await;
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    e.to_string(),
                )
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

                let (resp, _) =
                    translate::from_chat_response(response_id, &target.public_model, chat_resp);
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
    write_request_log(
        &state.db,
        LogInput {
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
        },
    )
    .await;
}

async fn write_request_log(db: &Db, mut input: LogInput) {
    let settings = match db.get_app_settings().await {
        Ok(settings) => settings,
        Err(_) => return,
    };
    if !settings.request_logging_enabled {
        return;
    }
    let max_chars = settings.log_error_max_chars.max(0) as usize;
    input.error = input.error.map(|e| e.chars().take(max_chars).collect());
    let _ = db.insert_request_log(input).await;
}

async fn authenticate_api_key(
    state: &AppState,
    headers: &HeaderMap,
) -> std::result::Result<db::ApiKeyRecord, Response> {
    let Some(raw) = bearer_token(headers) else {
        return Err(api_error(
            StatusCode::UNAUTHORIZED,
            "MISSING_API_KEY",
            "Missing API key",
        ));
    };
    let hash = state.crypto.hash_api_key(&raw);
    match state.db.find_api_key_by_hash(&hash).await {
        Ok(Some(record)) if record.enabled => Ok(record),
        Ok(Some(_)) => Err(api_error(
            StatusCode::UNAUTHORIZED,
            "DISABLED_API_KEY",
            "Disabled API key",
        )),
        Ok(None) => Err(api_error(
            StatusCode::UNAUTHORIZED,
            "INVALID_API_KEY",
            "Invalid API key",
        )),
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

fn choose_route_candidate(mut candidates: Vec<db::RouteCandidate>) -> Option<db::RouteCandidate> {
    match candidates.len() {
        0 => None,
        1 => candidates.pop(),
        len => {
            use argon2::password_hash::rand_core::{OsRng, RngCore};
            let mut rng = OsRng;
            let index = (rng.next_u64() as usize) % len;
            Some(candidates.swap_remove(index))
        }
    }
}

fn page_params(query: PageQuery) -> PageParams {
    PageParams::new(
        query.page.unwrap_or(1),
        query.page_size.unwrap_or(20),
        query.q,
    )
}

fn paginated<T>(items: Vec<T>, total: i64, params: &PageParams) -> Paginated<T> {
    let total_pages = if total == 0 {
        0
    } else {
        (total + params.page_size - 1) / params.page_size
    };
    Paginated {
        items,
        total,
        page: params.page,
        page_size: params.page_size,
        total_pages,
    }
}

fn validate_settings(settings: &AppSettings) -> std::result::Result<(), Response> {
    if !(0..=600).contains(&settings.upstream_timeout_seconds) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "BAD_REQUEST",
            "upstream_timeout_seconds must be between 0 and 600",
        ));
    }
    if !(100..=10_000).contains(&settings.log_error_max_chars) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "BAD_REQUEST",
            "log_error_max_chars must be between 100 and 10000",
        ));
    }
    Ok(())
}

fn api_key_view(state: &AppState, key: db::ApiKeyRecord, models: Vec<String>) -> ApiKeyView {
    let decrypted = key
        .encrypted_key
        .as_deref()
        .and_then(|value| state.crypto.decrypt(value).ok());
    ApiKeyView {
        id: key.id,
        name: key.name,
        enabled: key.enabled,
        created_at: key.created_at,
        last_used_at: key.last_used_at,
        masked_key: decrypted.as_deref().map(mask_api_key),
        key_recoverable: decrypted.is_some(),
        models,
    }
}

fn mask_api_key(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    if chars.len() <= 12 {
        return "••••".to_string();
    }
    let start: String = chars.iter().take(6).collect();
    let end: String = chars
        .iter()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{start}••••••{end}")
}

async fn current_admin(state: &AppState, headers: &HeaderMap) -> Result<Option<db::AdminUser>> {
    let Some(token) = session_cookie(headers) else {
        return Ok(None);
    };
    let hash = state.crypto.hash_api_key(&token);
    state.db.admin_for_session(&hash).await
}

async fn require_admin(
    state: &AppState,
    headers: &HeaderMap,
) -> std::result::Result<db::AdminUser, Response> {
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
        HeaderValue::from_static(
            "chat2responses_admin=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0",
        ),
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

async fn fetch_models_from_upstream(
    client: &Client,
    base_url: &str,
    api_key: &str,
) -> std::result::Result<UpstreamModels, Response> {
    let url = format!(
        "{}models",
        join_base_str(base_url.trim().trim_end_matches('/'))
    );
    let mut builder = client.get(url);
    if !api_key.trim().is_empty() {
        builder = builder.bearer_auth(api_key.trim());
    }
    match builder.send().await {
        Ok(r) if r.status().is_success() => match r.json::<serde_json::Value>().await {
            Ok(body) => {
                let data = model_list_from_body(&body);
                let mut model_configs: Vec<UpstreamModelInput> =
                    data.iter().filter_map(model_config_from_value).collect();
                model_configs.sort_by(|a, b| a.model.cmp(&b.model));
                model_configs.dedup_by(|a, b| a.model == b.model);
                let models = model_configs
                    .iter()
                    .map(|model| model.model.clone())
                    .collect();
                Ok(UpstreamModels {
                    data,
                    models,
                    model_configs,
                })
            }
            Err(e) => Err(internal_error(e)),
        },
        Ok(r) => {
            let status = r.status();
            let body = r.text().await.unwrap_or_default();
            Err((
                StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
                body,
            )
                .into_response())
        }
        Err(e) => Err(api_error(
            StatusCode::BAD_GATEWAY,
            "UPSTREAM_CONNECTION_ERROR",
            e.to_string(),
        )),
    }
}

fn model_list_from_body(body: &serde_json::Value) -> Vec<serde_json::Value> {
    body.get("data")
        .or_else(|| body.get("models"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

fn model_config_from_value(value: &serde_json::Value) -> Option<UpstreamModelInput> {
    let model = value
        .get("id")
        .and_then(|id| id.as_str())
        .or_else(|| value.as_str())?
        .trim();
    if model.is_empty() {
        return None;
    }
    let context_window = json_i64(
        value,
        &["context_window", "context_length", "max_context_window"],
    )
    .unwrap_or(128_000);
    let max_context_window =
        json_i64(value, &["max_context_window", "max_context_length"]).unwrap_or(context_window);
    Some(UpstreamModelInput {
        model: model.to_string(),
        enabled: true,
        context_window,
        max_context_window,
        supports_parallel_tool_calls: json_bool(
            value,
            &["supports_parallel_tool_calls", "parallel_tool_calls"],
        )
        .unwrap_or(true),
        supports_reasoning_summaries: json_bool(
            value,
            &["supports_reasoning_summaries", "reasoning_summaries"],
        )
        .unwrap_or(false),
    })
}

fn json_i64(value: &serde_json::Value, keys: &[&str]) -> Option<i64> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(|item| item.as_i64()))
}

fn json_bool(value: &serde_json::Value, keys: &[&str]) -> Option<bool> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(|item| item.as_bool()))
}

fn merge_model_configs(base: &mut [UpstreamModelInput], overrides: &[UpstreamModelInput]) {
    for item in base {
        if let Some(override_item) = overrides
            .iter()
            .find(|candidate| candidate.model.trim() == item.model)
        {
            item.enabled = override_item.enabled;
            item.context_window = override_item.context_window.max(1);
            item.max_context_window = override_item.max_context_window.max(1);
            item.supports_parallel_tool_calls = override_item.supports_parallel_tool_calls;
            item.supports_reasoning_summaries = override_item.supports_reasoning_summaries;
        }
    }
}

async fn handle_fallback(req: Request<Body>) -> Response {
    if req.method() == Method::GET && req.uri().path().starts_with("/admin/") {
        return admin_index().await;
    }
    warn!("unhandled {} {}", req.method(), req.uri().path());
    api_error(StatusCode::NOT_FOUND, "NOT_FOUND", "Not found")
}

fn api_error(status: StatusCode, code: &'static str, message: impl Into<String>) -> Response {
    (
        status,
        Json(ApiErrorBody {
            code,
            message: message.into(),
        }),
    )
        .into_response()
}

fn internal_error(err: impl std::fmt::Display) -> Response {
    error!("{err}");
    api_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "INTERNAL_ERROR",
        err.to_string(),
    )
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
        assert_eq!(
            join_base_str("https://api.example.com/v1"),
            "https://api.example.com/v1/"
        );
    }

    #[test]
    fn test_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer cr_test"),
        );
        assert_eq!(bearer_token(&headers).as_deref(), Some("cr_test"));
    }

    #[test]
    fn test_model_list_from_body_supports_data_and_models() {
        let data = serde_json::json!({"data":[{"id":"a"}]});
        let models = serde_json::json!({"models":[{"id":"b"}]});
        assert_eq!(model_list_from_body(&data)[0]["id"], "a");
        assert_eq!(model_list_from_body(&models)[0]["id"], "b");
    }

    #[test]
    fn test_validate_settings_bounds() {
        assert!(validate_settings(&AppSettings::default()).is_ok());
        assert!(validate_settings(&AppSettings {
            request_logging_enabled: true,
            upstream_timeout_seconds: 601,
            log_error_max_chars: 500,
        })
        .is_err());
        assert!(validate_settings(&AppSettings {
            request_logging_enabled: true,
            upstream_timeout_seconds: 60,
            log_error_max_chars: 99,
        })
        .is_err());
    }

    #[tokio::test]
    async fn test_write_request_log_obeys_settings() {
        let db = Db::connect("sqlite::memory:").await.unwrap();

        write_request_log(&db, test_log_input("x".repeat(200))).await;
        let (rows, total) = db
            .list_request_logs_paged(&PageParams::new(1, 10, None))
            .await
            .unwrap();
        assert_eq!(total, 0);
        assert!(rows.is_empty());

        db.save_app_settings(&AppSettings {
            request_logging_enabled: true,
            upstream_timeout_seconds: 0,
            log_error_max_chars: 100,
        })
        .await
        .unwrap();
        write_request_log(&db, test_log_input("x".repeat(200))).await;
        let (rows, total) = db
            .list_request_logs_paged(&PageParams::new(1, 10, None))
            .await
            .unwrap();
        assert_eq!(total, 1);
        assert_eq!(rows[0].error.as_ref().unwrap().chars().count(), 100);
    }

    fn test_log_input(error: String) -> LogInput {
        LogInput {
            api_key_id: None,
            public_model: Some("public".into()),
            upstream_id: None,
            upstream_model: None,
            status_code: 500,
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            error: Some(error),
            duration_ms: 10,
        }
    }

    #[tokio::test]
    async fn test_api_error_shape() {
        let resp = api_error(
            StatusCode::UNAUTHORIZED,
            "INVALID_API_KEY",
            "Invalid API key",
        );
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "code": "INVALID_API_KEY",
                "message": "Invalid API key"
            })
        );
    }
}
