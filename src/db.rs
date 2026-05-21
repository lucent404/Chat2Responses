use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    FromRow, SqlitePool,
};
use std::{collections::BTreeSet, str::FromStr};

#[derive(Clone)]
pub struct Db {
    pool: SqlitePool,
}

#[derive(Debug, Serialize, FromRow)]
pub struct AdminUser {
    pub id: i64,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Upstream {
    pub id: i64,
    pub name: String,
    pub base_url: String,
    #[serde(skip_serializing)]
    pub encrypted_api_key: String,
    pub enabled: bool,
    pub model_count: i64,
    pub enabled_model_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, FromRow)]
pub struct UpstreamModel {
    pub id: i64,
    pub upstream_id: i64,
    pub upstream_name: String,
    pub model: String,
    pub enabled: bool,
    pub context_window: i64,
    pub max_context_window: i64,
    pub supports_parallel_tool_calls: bool,
    pub supports_reasoning_summaries: bool,
    pub supports_image_input: bool,
    pub last_seen_at: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpstreamModelInput {
    pub model: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_context")]
    pub context_window: i64,
    #[serde(default = "default_context")]
    pub max_context_window: i64,
    #[serde(default = "default_true")]
    pub supports_parallel_tool_calls: bool,
    #[serde(default)]
    pub supports_reasoning_summaries: bool,
    #[serde(default)]
    pub supports_image_input: bool,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ModelRoute {
    pub id: i64,
    pub public_model: String,
    pub upstream_id: i64,
    pub upstream_name: String,
    pub upstream_model: String,
    pub context_window: i64,
    pub max_context_window: i64,
    pub supports_parallel_tool_calls: bool,
    pub supports_reasoning_summaries: bool,
    pub supports_image_input: bool,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ApiKeyRecord {
    pub id: i64,
    pub name: String,
    #[serde(skip_serializing)]
    pub _key_hash: String,
    #[serde(skip_serializing)]
    pub encrypted_key: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct AvailableModel {
    pub id: String,
    pub source: String,
    pub owner: String,
    pub candidate_count: i64,
    pub context_window: i64,
    pub max_context_window: i64,
    pub supports_parallel_tool_calls: bool,
    pub supports_reasoning_summaries: bool,
    pub supports_image_input: bool,
}

#[derive(Debug, FromRow)]
pub struct RouteCandidate {
    pub public_model: String,
    pub upstream_id: i64,
    pub _upstream_name: String,
    pub upstream_model: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct RequestLog {
    pub id: i64,
    pub api_key_id: Option<i64>,
    pub api_key_name: Option<String>,
    pub public_model: Option<String>,
    pub upstream_id: Option<i64>,
    pub upstream_name: Option<String>,
    pub upstream_model: Option<String>,
    pub status_code: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub error: Option<String>,
    pub duration_ms: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct AppSettings {
    pub request_logging_enabled: bool,
    pub upstream_timeout_seconds: i64,
    pub log_error_max_chars: i64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            request_logging_enabled: false,
            upstream_timeout_seconds: 0,
            log_error_max_chars: 500,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PageParams {
    pub page: i64,
    pub page_size: i64,
    pub q: Option<String>,
}

impl PageParams {
    pub fn new(page: i64, page_size: i64, q: Option<String>) -> Self {
        let q = q
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        Self {
            page: page.max(1),
            page_size: page_size.clamp(1, 100),
            q,
        }
    }

    fn offset(&self) -> i64 {
        (self.page - 1) * self.page_size
    }
}

#[derive(Debug, Deserialize)]
pub struct UpstreamInput {
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub models: Option<Vec<String>>,
    #[serde(default)]
    pub model_configs: Option<Vec<UpstreamModelInput>>,
}

#[derive(Debug, Deserialize)]
pub struct ModelRouteInput {
    pub public_model: String,
    pub upstream_id: i64,
    pub upstream_model: String,
    #[serde(default = "default_context")]
    pub context_window: i64,
    #[serde(default = "default_context")]
    pub max_context_window: i64,
    #[serde(default = "default_true")]
    pub supports_parallel_tool_calls: bool,
    #[serde(default)]
    pub supports_reasoning_summaries: bool,
    #[serde(default)]
    pub supports_image_input: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct ApiKeyInput {
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub models: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct LogInput {
    pub api_key_id: Option<i64>,
    pub public_model: Option<String>,
    pub upstream_id: Option<i64>,
    pub upstream_model: Option<String>,
    pub status_code: u16,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub error: Option<String>,
    pub duration_ms: i64,
}

fn default_true() -> bool {
    true
}

fn default_context() -> i64 {
    128_000
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn normalize_models(models: &[String]) -> Vec<String> {
    models
        .iter()
        .map(|model| model.trim())
        .filter(|model| !model.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(String::from)
        .collect()
}

fn normalize_model_inputs(models: &[UpstreamModelInput]) -> Vec<UpstreamModelInput> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for item in models {
        let model = item.model.trim();
        if model.is_empty() || !seen.insert(model.to_string()) {
            continue;
        }
        normalized.push(UpstreamModelInput {
            model: model.to_string(),
            enabled: item.enabled,
            context_window: item.context_window.max(1),
            max_context_window: item.max_context_window.max(1),
            supports_parallel_tool_calls: item.supports_parallel_tool_calls,
            supports_reasoning_summaries: item.supports_reasoning_summaries,
            supports_image_input: item.supports_image_input,
        });
    }
    normalized
}

impl Db {
    pub async fn connect(database_url: &str) -> Result<Self> {
        if let Some(path) = database_url.strip_prefix("sqlite://") {
            if path != ":memory:" {
                if let Some(parent) = std::path::Path::new(path).parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
            }
        }
        let options = SqliteConnectOptions::from_str(database_url)?.create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        let db = Self { pool };
        db.migrate().await?;
        Ok(db)
    }

    async fn migrate(&self) -> Result<()> {
        let statements = [
            r#"
            CREATE TABLE IF NOT EXISTS admin_users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                created_at TEXT NOT NULL
            )
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS admin_sessions (
                token_hash TEXT PRIMARY KEY,
                admin_id INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                FOREIGN KEY(admin_id) REFERENCES admin_users(id) ON DELETE CASCADE
            )
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS upstreams (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                base_url TEXT NOT NULL,
                encrypted_api_key TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS model_routes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                public_model TEXT NOT NULL,
                upstream_id INTEGER NOT NULL,
                upstream_model TEXT NOT NULL,
                context_window INTEGER NOT NULL DEFAULT 128000,
                max_context_window INTEGER NOT NULL DEFAULT 128000,
                supports_parallel_tool_calls INTEGER NOT NULL DEFAULT 1,
                supports_reasoning_summaries INTEGER NOT NULL DEFAULT 0,
                supports_image_input INTEGER NOT NULL DEFAULT 0,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY(upstream_id) REFERENCES upstreams(id) ON DELETE CASCADE
            )
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS upstream_models (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                upstream_id INTEGER NOT NULL,
                model TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                context_window INTEGER NOT NULL DEFAULT 128000,
                max_context_window INTEGER NOT NULL DEFAULT 128000,
                supports_parallel_tool_calls INTEGER NOT NULL DEFAULT 1,
                supports_reasoning_summaries INTEGER NOT NULL DEFAULT 0,
                supports_image_input INTEGER NOT NULL DEFAULT 0,
                last_seen_at TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(upstream_id, model),
                FOREIGN KEY(upstream_id) REFERENCES upstreams(id) ON DELETE CASCADE
            )
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS api_keys (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                key_hash TEXT NOT NULL UNIQUE,
                encrypted_key TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                last_used_at TEXT
            )
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS api_key_models (
                api_key_id INTEGER NOT NULL,
                model TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY(api_key_id, model),
                FOREIGN KEY(api_key_id) REFERENCES api_keys(id) ON DELETE CASCADE
            )
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS request_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                api_key_id INTEGER,
                public_model TEXT,
                upstream_id INTEGER,
                upstream_model TEXT,
                status_code INTEGER NOT NULL,
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                total_tokens INTEGER NOT NULL DEFAULT 0,
                error TEXT,
                duration_ms INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY(api_key_id) REFERENCES api_keys(id) ON DELETE SET NULL,
                FOREIGN KEY(upstream_id) REFERENCES upstreams(id) ON DELETE SET NULL
            )
            "#,
        ];
        for statement in statements {
            sqlx::query(statement).execute(&self.pool).await?;
        }
        self.ensure_api_key_encrypted_column().await?;
        self.ensure_upstream_model_metadata_columns().await?;
        self.ensure_model_route_metadata_columns().await?;
        self.ensure_model_routes_without_unique_public_model()
            .await?;
        Ok(())
    }

    async fn ensure_api_key_encrypted_column(&self) -> Result<()> {
        let exists: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM pragma_table_info('api_keys') WHERE name = 'encrypted_key'",
        )
        .fetch_optional(&self.pool)
        .await?;
        if exists.is_none() {
            sqlx::query("ALTER TABLE api_keys ADD COLUMN encrypted_key TEXT")
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    async fn ensure_upstream_model_metadata_columns(&self) -> Result<()> {
        for (name, definition) in [
            ("context_window", "INTEGER NOT NULL DEFAULT 128000"),
            ("max_context_window", "INTEGER NOT NULL DEFAULT 128000"),
            ("supports_parallel_tool_calls", "INTEGER NOT NULL DEFAULT 1"),
            ("supports_reasoning_summaries", "INTEGER NOT NULL DEFAULT 0"),
            ("supports_image_input", "INTEGER NOT NULL DEFAULT 0"),
        ] {
            let exists: Option<i64> = sqlx::query_scalar(
                "SELECT 1 FROM pragma_table_info('upstream_models') WHERE name = ?",
            )
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;
            if exists.is_none() {
                sqlx::query(&format!(
                    "ALTER TABLE upstream_models ADD COLUMN {name} {definition}"
                ))
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }

    async fn ensure_model_route_metadata_columns(&self) -> Result<()> {
        for (name, definition) in [
            ("context_window", "INTEGER NOT NULL DEFAULT 128000"),
            ("max_context_window", "INTEGER NOT NULL DEFAULT 128000"),
            ("supports_parallel_tool_calls", "INTEGER NOT NULL DEFAULT 1"),
            ("supports_reasoning_summaries", "INTEGER NOT NULL DEFAULT 0"),
            ("supports_image_input", "INTEGER NOT NULL DEFAULT 0"),
        ] {
            let exists: Option<i64> = sqlx::query_scalar(
                "SELECT 1 FROM pragma_table_info('model_routes') WHERE name = ?",
            )
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;
            if exists.is_none() {
                sqlx::query(&format!(
                    "ALTER TABLE model_routes ADD COLUMN {name} {definition}"
                ))
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }

    async fn ensure_model_routes_without_unique_public_model(&self) -> Result<()> {
        let create_sql: Option<String> = sqlx::query_scalar(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'model_routes'",
        )
        .fetch_optional(&self.pool)
        .await?;
        let Some(create_sql) = create_sql else {
            return Ok(());
        };
        if !create_sql
            .to_uppercase()
            .contains("PUBLIC_MODEL TEXT NOT NULL UNIQUE")
        {
            return Ok(());
        }

        sqlx::query(
            r#"
            CREATE TABLE model_routes_new (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                public_model TEXT NOT NULL,
                upstream_id INTEGER NOT NULL,
                upstream_model TEXT NOT NULL,
                context_window INTEGER NOT NULL DEFAULT 128000,
                max_context_window INTEGER NOT NULL DEFAULT 128000,
                supports_parallel_tool_calls INTEGER NOT NULL DEFAULT 1,
                supports_reasoning_summaries INTEGER NOT NULL DEFAULT 0,
                supports_image_input INTEGER NOT NULL DEFAULT 0,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY(upstream_id) REFERENCES upstreams(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            r#"
            INSERT INTO model_routes_new
            (id, public_model, upstream_id, upstream_model, context_window, max_context_window,
             supports_parallel_tool_calls, supports_reasoning_summaries, supports_image_input, enabled, created_at, updated_at)
            SELECT id, public_model, upstream_id, upstream_model, context_window, max_context_window,
                   supports_parallel_tool_calls, supports_reasoning_summaries, 0, enabled, created_at, updated_at
            FROM model_routes
            "#,
        )
        .execute(&self.pool)
        .await?;
        sqlx::query("DROP TABLE model_routes")
            .execute(&self.pool)
            .await?;
        sqlx::query("ALTER TABLE model_routes_new RENAME TO model_routes")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn has_admin(&self) -> Result<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admin_users")
            .fetch_one(&self.pool)
            .await?;
        Ok(count > 0)
    }

    pub async fn create_admin(&self, username: &str, password_hash: &str) -> Result<AdminUser> {
        let created_at = now();
        let id = sqlx::query(
            "INSERT INTO admin_users (username, password_hash, created_at) VALUES (?, ?, ?)",
        )
        .bind(username)
        .bind(password_hash)
        .bind(&created_at)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();
        Ok(AdminUser {
            id,
            username: username.to_string(),
            password_hash: password_hash.to_string(),
            created_at,
        })
    }

    pub async fn find_admin_by_username(&self, username: &str) -> Result<Option<AdminUser>> {
        Ok(sqlx::query_as::<_, AdminUser>(
            "SELECT id, username, password_hash, created_at FROM admin_users WHERE username = ?",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn create_admin_session(
        &self,
        token_hash: &str,
        admin_id: i64,
        expires_at: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO admin_sessions (token_hash, admin_id, created_at, expires_at) VALUES (?, ?, ?, ?)",
        )
        .bind(token_hash)
        .bind(admin_id)
        .bind(now())
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn admin_for_session(&self, token_hash: &str) -> Result<Option<AdminUser>> {
        Ok(sqlx::query_as::<_, AdminUser>(
            r#"
            SELECT a.id, a.username, a.password_hash, a.created_at
            FROM admin_sessions s
            JOIN admin_users a ON a.id = s.admin_id
            WHERE s.token_hash = ? AND s.expires_at > ?
            "#,
        )
        .bind(token_hash)
        .bind(now())
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn delete_admin_session(&self, token_hash: &str) -> Result<()> {
        sqlx::query("DELETE FROM admin_sessions WHERE token_hash = ?")
            .bind(token_hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_app_settings(&self) -> Result<AppSettings> {
        let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM app_settings")
            .fetch_all(&self.pool)
            .await?;
        let mut settings = AppSettings::default();
        for (key, value) in rows {
            match key.as_str() {
                "request_logging_enabled" => {
                    settings.request_logging_enabled = matches!(value.as_str(), "true" | "1");
                }
                "upstream_timeout_seconds" => {
                    if let Ok(parsed) = value.parse::<i64>() {
                        settings.upstream_timeout_seconds = parsed;
                    }
                }
                "log_error_max_chars" => {
                    if let Ok(parsed) = value.parse::<i64>() {
                        settings.log_error_max_chars = parsed;
                    }
                }
                _ => {}
            }
        }
        Ok(settings)
    }

    pub async fn save_app_settings(&self, settings: &AppSettings) -> Result<AppSettings> {
        let ts = now();
        for (key, value) in [
            (
                "request_logging_enabled",
                settings.request_logging_enabled.to_string(),
            ),
            (
                "upstream_timeout_seconds",
                settings.upstream_timeout_seconds.to_string(),
            ),
            (
                "log_error_max_chars",
                settings.log_error_max_chars.to_string(),
            ),
        ] {
            sqlx::query(
                r#"
                INSERT INTO app_settings (key, value, updated_at)
                VALUES (?, ?, ?)
                ON CONFLICT(key) DO UPDATE SET
                    value = excluded.value,
                    updated_at = excluded.updated_at
                "#,
            )
            .bind(key)
            .bind(value)
            .bind(&ts)
            .execute(&self.pool)
            .await?;
        }
        self.get_app_settings().await
    }

    pub async fn list_upstreams_paged(&self, params: &PageParams) -> Result<(Vec<Upstream>, i64)> {
        let like = params.q.as_ref().map(|q| format!("%{q}%"));
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM upstreams u
            WHERE (? IS NULL OR u.name LIKE ? OR u.base_url LIKE ?)
            "#,
        )
        .bind(like.as_deref())
        .bind(like.as_deref())
        .bind(like.as_deref())
        .fetch_one(&self.pool)
        .await?;
        let rows = sqlx::query_as::<_, Upstream>(
            r#"
            SELECT u.id, u.name, u.base_url, u.encrypted_api_key, u.enabled != 0 AS enabled,
                   COUNT(m.id) AS model_count,
                   COALESCE(SUM(CASE WHEN m.enabled = 1 THEN 1 ELSE 0 END), 0) AS enabled_model_count,
                   u.created_at, u.updated_at
            FROM upstreams u
            LEFT JOIN upstream_models m ON m.upstream_id = u.id
            WHERE (? IS NULL OR u.name LIKE ? OR u.base_url LIKE ?)
            GROUP BY u.id
            ORDER BY u.id DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(like.as_deref())
        .bind(like.as_deref())
        .bind(like.as_deref())
        .bind(params.page_size)
        .bind(params.offset())
        .fetch_all(&self.pool)
        .await?;
        Ok((rows, total))
    }

    pub async fn get_upstream(&self, id: i64) -> Result<Option<Upstream>> {
        Ok(sqlx::query_as::<_, Upstream>(
            r#"
            SELECT u.id, u.name, u.base_url, u.encrypted_api_key, u.enabled != 0 AS enabled,
                   COUNT(m.id) AS model_count,
                   COALESCE(SUM(CASE WHEN m.enabled = 1 THEN 1 ELSE 0 END), 0) AS enabled_model_count,
                   u.created_at, u.updated_at
            FROM upstreams u
            LEFT JOIN upstream_models m ON m.upstream_id = u.id
            WHERE u.id = ?
            GROUP BY u.id
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn create_upstream(
        &self,
        input: &UpstreamInput,
        encrypted_api_key: String,
    ) -> Result<Upstream> {
        let ts = now();
        let id = sqlx::query(
            "INSERT INTO upstreams (name, base_url, encrypted_api_key, enabled, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(input.name.trim())
        .bind(input.base_url.trim().trim_end_matches('/'))
        .bind(&encrypted_api_key)
        .bind(input.enabled)
        .bind(&ts)
        .bind(&ts)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();
        Ok(Upstream {
            id,
            name: input.name.trim().to_string(),
            base_url: input.base_url.trim().trim_end_matches('/').to_string(),
            encrypted_api_key,
            enabled: input.enabled,
            model_count: 0,
            enabled_model_count: 0,
            created_at: ts.clone(),
            updated_at: ts,
        })
    }

    pub async fn update_upstream(
        &self,
        id: i64,
        input: &UpstreamInput,
        encrypted_api_key: Option<String>,
    ) -> Result<Option<Upstream>> {
        let Some(existing) = self.get_upstream(id).await? else {
            return Ok(None);
        };
        let key = encrypted_api_key.unwrap_or(existing.encrypted_api_key);
        sqlx::query(
            "UPDATE upstreams SET name = ?, base_url = ?, encrypted_api_key = ?, enabled = ?, updated_at = ? WHERE id = ?",
        )
        .bind(input.name.trim())
        .bind(input.base_url.trim().trim_end_matches('/'))
        .bind(&key)
        .bind(input.enabled)
        .bind(now())
        .bind(id)
        .execute(&self.pool)
        .await?;
        self.get_upstream(id).await
    }

    pub async fn delete_upstream(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM upstreams WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn upsert_upstream_models(
        &self,
        upstream_id: i64,
        discovered: &[UpstreamModelInput],
        selected: Option<&[String]>,
    ) -> Result<()> {
        let ts = now();
        let discovered = normalize_model_inputs(discovered);
        let selected = selected.map(normalize_models);
        let enabled_models = selected.as_ref().filter(|items| !items.is_empty());

        for item in &discovered {
            let enabled = enabled_models.map_or(item.enabled, |items| items.contains(&item.model));
            sqlx::query(
                r#"
                INSERT INTO upstream_models
                (upstream_id, model, enabled, context_window, max_context_window,
                 supports_parallel_tool_calls, supports_reasoning_summaries, supports_image_input,
                 last_seen_at, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(upstream_id, model) DO UPDATE SET
                    enabled = excluded.enabled,
                    context_window = excluded.context_window,
                    max_context_window = excluded.max_context_window,
                    supports_parallel_tool_calls = excluded.supports_parallel_tool_calls,
                    supports_reasoning_summaries = excluded.supports_reasoning_summaries,
                    supports_image_input = excluded.supports_image_input,
                    last_seen_at = excluded.last_seen_at,
                    updated_at = excluded.updated_at
                "#,
            )
            .bind(upstream_id)
            .bind(&item.model)
            .bind(enabled)
            .bind(item.context_window)
            .bind(item.max_context_window)
            .bind(item.supports_parallel_tool_calls)
            .bind(item.supports_reasoning_summaries)
            .bind(item.supports_image_input)
            .bind(&ts)
            .bind(&ts)
            .bind(&ts)
            .execute(&self.pool)
            .await?;
        }

        if let Some(enabled_models) = enabled_models {
            sqlx::query(
                r#"
                UPDATE upstream_models
                SET enabled = 0, updated_at = ?
                WHERE upstream_id = ? AND model NOT IN (
                    SELECT value FROM json_each(?)
                )
                "#,
            )
            .bind(&ts)
            .bind(upstream_id)
            .bind(serde_json::to_string(&enabled_models)?)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn sync_upstream_model_inventory(
        &self,
        upstream_id: i64,
        discovered: &[UpstreamModelInput],
    ) -> Result<()> {
        let ts = now();
        for item in normalize_model_inputs(discovered) {
            sqlx::query(
                r#"
                INSERT INTO upstream_models
                (upstream_id, model, enabled, context_window, max_context_window,
                 supports_parallel_tool_calls, supports_reasoning_summaries, supports_image_input,
                 last_seen_at, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(upstream_id, model) DO UPDATE SET
                    context_window = excluded.context_window,
                    max_context_window = excluded.max_context_window,
                    supports_parallel_tool_calls = excluded.supports_parallel_tool_calls,
                    supports_reasoning_summaries = excluded.supports_reasoning_summaries,
                    supports_image_input = excluded.supports_image_input,
                    last_seen_at = excluded.last_seen_at,
                    updated_at = excluded.updated_at
                "#,
            )
            .bind(upstream_id)
            .bind(&item.model)
            .bind(item.enabled)
            .bind(item.context_window)
            .bind(item.max_context_window)
            .bind(item.supports_parallel_tool_calls)
            .bind(item.supports_reasoning_summaries)
            .bind(item.supports_image_input)
            .bind(&ts)
            .bind(&ts)
            .bind(&ts)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn set_upstream_enabled_models(
        &self,
        upstream_id: i64,
        selected: Option<&[String]>,
    ) -> Result<()> {
        let Some(selected) = selected else {
            return Ok(());
        };
        let selected = normalize_models(selected);
        let ts = now();
        if selected.is_empty() {
            sqlx::query(
                "UPDATE upstream_models SET enabled = 1, updated_at = ? WHERE upstream_id = ?",
            )
            .bind(&ts)
            .bind(upstream_id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                r#"
                UPDATE upstream_models
                SET enabled = CASE WHEN model IN (SELECT value FROM json_each(?)) THEN 1 ELSE 0 END,
                    updated_at = ?
                WHERE upstream_id = ?
                "#,
            )
            .bind(serde_json::to_string(&selected)?)
            .bind(&ts)
            .bind(upstream_id)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn list_local_upstream_models(&self, upstream_id: i64) -> Result<Vec<UpstreamModel>> {
        Ok(sqlx::query_as::<_, UpstreamModel>(
            r#"
            SELECT m.id, m.upstream_id, u.name AS upstream_name, m.model,
                   m.enabled != 0 AS enabled,
                   m.context_window, m.max_context_window,
                   m.supports_parallel_tool_calls != 0 AS supports_parallel_tool_calls,
                   m.supports_reasoning_summaries != 0 AS supports_reasoning_summaries,
                   m.supports_image_input != 0 AS supports_image_input,
                   m.last_seen_at, m.created_at, m.updated_at
            FROM upstream_models m
            JOIN upstreams u ON u.id = m.upstream_id
            WHERE m.upstream_id = ?
            ORDER BY m.model
            "#,
        )
        .bind(upstream_id)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn save_upstream_models(
        &self,
        upstream_id: i64,
        models: &[UpstreamModelInput],
    ) -> Result<()> {
        let ts = now();
        for item in normalize_model_inputs(models) {
            sqlx::query(
                r#"
                INSERT INTO upstream_models
                (upstream_id, model, enabled, context_window, max_context_window,
                 supports_parallel_tool_calls, supports_reasoning_summaries, supports_image_input,
                 last_seen_at, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(upstream_id, model) DO UPDATE SET
                    enabled = excluded.enabled,
                    context_window = excluded.context_window,
                    max_context_window = excluded.max_context_window,
                    supports_parallel_tool_calls = excluded.supports_parallel_tool_calls,
                    supports_reasoning_summaries = excluded.supports_reasoning_summaries,
                    supports_image_input = excluded.supports_image_input,
                    updated_at = excluded.updated_at
                "#,
            )
            .bind(upstream_id)
            .bind(&item.model)
            .bind(item.enabled)
            .bind(item.context_window)
            .bind(item.max_context_window)
            .bind(item.supports_parallel_tool_calls)
            .bind(item.supports_reasoning_summaries)
            .bind(item.supports_image_input)
            .bind(&ts)
            .bind(&ts)
            .bind(&ts)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn list_model_routes_paged(
        &self,
        params: &PageParams,
    ) -> Result<(Vec<ModelRoute>, i64)> {
        let like = params.q.as_ref().map(|q| format!("%{q}%"));
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM model_routes m
            JOIN upstreams u ON u.id = m.upstream_id
            WHERE (? IS NULL OR m.public_model LIKE ? OR u.name LIKE ? OR m.upstream_model LIKE ?)
            "#,
        )
        .bind(like.as_deref())
        .bind(like.as_deref())
        .bind(like.as_deref())
        .bind(like.as_deref())
        .fetch_one(&self.pool)
        .await?;
        let rows = sqlx::query_as::<_, ModelRoute>(
            r#"
            SELECT m.id, m.public_model, m.upstream_id, u.name AS upstream_name, m.upstream_model,
                   m.context_window, m.max_context_window,
                   m.supports_parallel_tool_calls != 0 AS supports_parallel_tool_calls,
                   m.supports_reasoning_summaries != 0 AS supports_reasoning_summaries,
                   m.supports_image_input != 0 AS supports_image_input,
                   m.enabled != 0 AS enabled, m.created_at, m.updated_at
            FROM model_routes m
            JOIN upstreams u ON u.id = m.upstream_id
            WHERE (? IS NULL OR m.public_model LIKE ? OR u.name LIKE ? OR m.upstream_model LIKE ?)
            ORDER BY m.public_model, m.id DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(like.as_deref())
        .bind(like.as_deref())
        .bind(like.as_deref())
        .bind(like.as_deref())
        .bind(params.page_size)
        .bind(params.offset())
        .fetch_all(&self.pool)
        .await?;
        Ok((rows, total))
    }

    pub async fn create_model_route(&self, input: &ModelRouteInput) -> Result<ModelRoute> {
        let ts = now();
        let inherited = self
            .get_local_upstream_model(input.upstream_id, input.upstream_model.trim())
            .await?;
        let context_window = if input.context_window == default_context() {
            inherited
                .as_ref()
                .map(|model| model.context_window)
                .unwrap_or(input.context_window)
        } else {
            input.context_window
        };
        let max_context_window = if input.max_context_window == default_context() {
            inherited
                .as_ref()
                .map(|model| model.max_context_window)
                .unwrap_or(input.max_context_window)
        } else {
            input.max_context_window
        };
        let supports_parallel_tool_calls = inherited
            .as_ref()
            .map(|model| model.supports_parallel_tool_calls)
            .unwrap_or(input.supports_parallel_tool_calls);
        let supports_reasoning_summaries = if input.supports_reasoning_summaries {
            true
        } else {
            inherited
                .as_ref()
                .map(|model| model.supports_reasoning_summaries)
                .unwrap_or(false)
        };
        let supports_image_input = if input.supports_image_input {
            true
        } else {
            inherited
                .as_ref()
                .map(|model| model.supports_image_input)
                .unwrap_or(false)
        };
        let id = sqlx::query(
            r#"
            INSERT INTO model_routes
            (public_model, upstream_id, upstream_model, context_window, max_context_window,
             supports_parallel_tool_calls, supports_reasoning_summaries, supports_image_input,
             enabled, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(input.public_model.trim())
        .bind(input.upstream_id)
        .bind(input.upstream_model.trim())
        .bind(context_window)
        .bind(max_context_window)
        .bind(supports_parallel_tool_calls)
        .bind(supports_reasoning_summaries)
        .bind(supports_image_input)
        .bind(input.enabled)
        .bind(&ts)
        .bind(&ts)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();
        self.get_model_route(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("created model route missing"))
    }

    pub async fn get_local_upstream_model(
        &self,
        upstream_id: i64,
        model: &str,
    ) -> Result<Option<UpstreamModel>> {
        Ok(sqlx::query_as::<_, UpstreamModel>(
            r#"
            SELECT m.id, m.upstream_id, u.name AS upstream_name, m.model,
                   m.enabled != 0 AS enabled,
                   m.context_window, m.max_context_window,
                   m.supports_parallel_tool_calls != 0 AS supports_parallel_tool_calls,
                   m.supports_reasoning_summaries != 0 AS supports_reasoning_summaries,
                   m.supports_image_input != 0 AS supports_image_input,
                   m.last_seen_at, m.created_at, m.updated_at
            FROM upstream_models m
            JOIN upstreams u ON u.id = m.upstream_id
            WHERE m.upstream_id = ? AND m.model = ?
            "#,
        )
        .bind(upstream_id)
        .bind(model)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn get_model_route(&self, id: i64) -> Result<Option<ModelRoute>> {
        Ok(sqlx::query_as::<_, ModelRoute>(
            r#"
            SELECT m.id, m.public_model, m.upstream_id, u.name AS upstream_name, m.upstream_model,
                   m.context_window, m.max_context_window,
                   m.supports_parallel_tool_calls != 0 AS supports_parallel_tool_calls,
                   m.supports_reasoning_summaries != 0 AS supports_reasoning_summaries,
                   m.supports_image_input != 0 AS supports_image_input,
                   m.enabled != 0 AS enabled, m.created_at, m.updated_at
            FROM model_routes m JOIN upstreams u ON u.id = m.upstream_id
            WHERE m.id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn update_model_route(
        &self,
        id: i64,
        input: &ModelRouteInput,
    ) -> Result<Option<ModelRoute>> {
        sqlx::query(
            r#"
            UPDATE model_routes
            SET public_model = ?, upstream_id = ?, upstream_model = ?, context_window = ?,
                max_context_window = ?, supports_parallel_tool_calls = ?,
                supports_reasoning_summaries = ?, supports_image_input = ?,
                enabled = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(input.public_model.trim())
        .bind(input.upstream_id)
        .bind(input.upstream_model.trim())
        .bind(input.context_window)
        .bind(input.max_context_window)
        .bind(input.supports_parallel_tool_calls)
        .bind(input.supports_reasoning_summaries)
        .bind(input.supports_image_input)
        .bind(input.enabled)
        .bind(now())
        .bind(id)
        .execute(&self.pool)
        .await?;
        self.get_model_route(id).await
    }

    pub async fn delete_model_route(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM model_routes WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn create_api_key(
        &self,
        input: &ApiKeyInput,
        key_hash: &str,
        encrypted_key: String,
    ) -> Result<ApiKeyRecord> {
        let ts = now();
        let id = sqlx::query(
            "INSERT INTO api_keys (name, key_hash, encrypted_key, enabled, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(input.name.trim())
        .bind(key_hash)
        .bind(&encrypted_key)
        .bind(input.enabled)
        .bind(&ts)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();
        self.set_api_key_models(id, input.models.as_deref()).await?;
        Ok(ApiKeyRecord {
            id,
            name: input.name.trim().to_string(),
            _key_hash: key_hash.to_string(),
            encrypted_key: Some(encrypted_key),
            enabled: input.enabled,
            created_at: ts,
            last_used_at: None,
        })
    }

    pub async fn list_api_keys_paged(
        &self,
        params: &PageParams,
    ) -> Result<(Vec<(ApiKeyRecord, Vec<String>)>, i64)> {
        let like = params.q.as_ref().map(|q| format!("%{q}%"));
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM api_keys
            WHERE (? IS NULL OR name LIKE ?)
            "#,
        )
        .bind(like.as_deref())
        .bind(like.as_deref())
        .fetch_one(&self.pool)
        .await?;
        let keys = sqlx::query_as::<_, ApiKeyRecord>(
            r#"
            SELECT id, name, key_hash AS _key_hash, encrypted_key, enabled != 0 AS enabled, created_at, last_used_at
            FROM api_keys
            WHERE (? IS NULL OR name LIKE ?)
            ORDER BY id DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(like.as_deref())
        .bind(like.as_deref())
        .bind(params.page_size)
        .bind(params.offset())
        .fetch_all(&self.pool)
        .await?;
        let mut rows = Vec::with_capacity(keys.len());
        for key in keys {
            let models = self.list_api_key_models(key.id).await?;
            rows.push((key, models));
        }
        Ok((rows, total))
    }

    pub async fn get_api_key(&self, id: i64) -> Result<Option<ApiKeyRecord>> {
        Ok(sqlx::query_as::<_, ApiKeyRecord>(
            "SELECT id, name, key_hash AS _key_hash, encrypted_key, enabled != 0 AS enabled, created_at, last_used_at FROM api_keys WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn find_api_key_by_hash(&self, key_hash: &str) -> Result<Option<ApiKeyRecord>> {
        Ok(sqlx::query_as::<_, ApiKeyRecord>(
            "SELECT id, name, key_hash AS _key_hash, encrypted_key, enabled != 0 AS enabled, created_at, last_used_at FROM api_keys WHERE key_hash = ?",
        )
        .bind(key_hash)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn set_api_key_enabled(&self, id: i64, enabled: bool) -> Result<()> {
        sqlx::query("UPDATE api_keys SET enabled = ? WHERE id = ?")
            .bind(enabled)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_api_key_models(
        &self,
        api_key_id: i64,
        models: Option<&[String]>,
    ) -> Result<()> {
        sqlx::query("DELETE FROM api_key_models WHERE api_key_id = ?")
            .bind(api_key_id)
            .execute(&self.pool)
            .await?;
        let Some(models) = models else {
            return Ok(());
        };
        let models = normalize_models(models);
        if models.is_empty() {
            return Ok(());
        }
        let ts = now();
        for model in models {
            sqlx::query(
                "INSERT INTO api_key_models (api_key_id, model, created_at) VALUES (?, ?, ?)",
            )
            .bind(api_key_id)
            .bind(model)
            .bind(&ts)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn list_api_key_models(&self, api_key_id: i64) -> Result<Vec<String>> {
        Ok(sqlx::query_scalar(
            "SELECT model FROM api_key_models WHERE api_key_id = ? ORDER BY model",
        )
        .bind(api_key_id)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn is_api_key_allowed_model(&self, api_key_id: i64, model: &str) -> Result<bool> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM api_key_models WHERE api_key_id = ?")
                .bind(api_key_id)
                .fetch_one(&self.pool)
                .await?;
        if count == 0 {
            return Ok(true);
        }
        let allowed: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM api_key_models WHERE api_key_id = ? AND model = ?")
                .bind(api_key_id)
                .bind(model)
                .fetch_optional(&self.pool)
                .await?;
        Ok(allowed.is_some())
    }

    pub async fn list_available_models_for_key(
        &self,
        api_key_id: Option<i64>,
    ) -> Result<Vec<AvailableModel>> {
        let mut rows = Vec::new();
        rows.extend(
            sqlx::query_as::<_, AvailableModel>(
                r#"
                SELECT m.model AS id, 'upstream' AS source, u.name AS owner, COUNT(*) AS candidate_count,
                       MAX(m.context_window) AS context_window,
                       MAX(m.max_context_window) AS max_context_window,
                       MAX(CASE WHEN m.supports_parallel_tool_calls THEN 1 ELSE 0 END) AS supports_parallel_tool_calls,
                       MAX(CASE WHEN m.supports_reasoning_summaries THEN 1 ELSE 0 END) AS supports_reasoning_summaries,
                       MAX(CASE WHEN m.supports_image_input THEN 1 ELSE 0 END) AS supports_image_input
                FROM upstream_models m JOIN upstreams u ON u.id = m.upstream_id
                WHERE m.enabled = 1 AND u.enabled = 1
                GROUP BY m.model, u.name
                "#,
            )
            .fetch_all(&self.pool)
            .await?,
        );
        rows.extend(
            sqlx::query_as::<_, AvailableModel>(
                r#"
                SELECT m.public_model AS id, 'mapping' AS source, u.name AS owner, COUNT(*) AS candidate_count,
                       MAX(m.context_window) AS context_window,
                       MAX(m.max_context_window) AS max_context_window,
                       MAX(CASE WHEN m.supports_parallel_tool_calls THEN 1 ELSE 0 END) AS supports_parallel_tool_calls,
                       MAX(CASE WHEN m.supports_reasoning_summaries THEN 1 ELSE 0 END) AS supports_reasoning_summaries,
                       MAX(CASE WHEN m.supports_image_input THEN 1 ELSE 0 END) AS supports_image_input
                FROM model_routes m
                JOIN upstreams u ON u.id = m.upstream_id
                JOIN upstream_models um ON um.upstream_id = m.upstream_id AND um.model = m.upstream_model
                WHERE m.enabled = 1 AND u.enabled = 1 AND um.enabled = 1
                GROUP BY m.public_model, u.name
                "#,
            )
            .fetch_all(&self.pool)
            .await?,
        );

        let mut merged = Vec::<AvailableModel>::new();
        for row in rows {
            if let Some(api_key_id) = api_key_id {
                if !self.is_api_key_allowed_model(api_key_id, &row.id).await? {
                    continue;
                }
            }
            if let Some(existing) = merged.iter_mut().find(|item| item.id == row.id) {
                existing.candidate_count += row.candidate_count;
                existing.context_window = existing.context_window.max(row.context_window);
                existing.max_context_window =
                    existing.max_context_window.max(row.max_context_window);
                existing.supports_parallel_tool_calls |= row.supports_parallel_tool_calls;
                existing.supports_reasoning_summaries |= row.supports_reasoning_summaries;
                existing.supports_image_input |= row.supports_image_input;
                if !existing.source.contains(&row.source) {
                    existing.source = format!("{},{}", existing.source, row.source);
                }
                if !existing.owner.split(", ").any(|owner| owner == row.owner) {
                    existing.owner = format!("{}, {}", existing.owner, row.owner);
                }
            } else {
                merged.push(row);
            }
        }
        merged.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(merged)
    }

    pub async fn list_route_candidates(
        &self,
        api_key_id: i64,
        public_model: &str,
    ) -> Result<Vec<RouteCandidate>> {
        if !self
            .is_api_key_allowed_model(api_key_id, public_model)
            .await?
        {
            return Ok(Vec::new());
        }
        let mut candidates = Vec::new();
        candidates.extend(
            sqlx::query_as::<_, RouteCandidate>(
                r#"
                SELECT m.model AS public_model, m.upstream_id, u.name AS _upstream_name, m.model AS upstream_model
                FROM upstream_models m JOIN upstreams u ON u.id = m.upstream_id
                WHERE m.model = ? AND m.enabled = 1 AND u.enabled = 1
                "#,
            )
            .bind(public_model)
            .fetch_all(&self.pool)
            .await?,
        );
        candidates.extend(
            sqlx::query_as::<_, RouteCandidate>(
                r#"
                SELECT m.public_model, m.upstream_id, u.name AS _upstream_name, m.upstream_model
                FROM model_routes m
                JOIN upstreams u ON u.id = m.upstream_id
                JOIN upstream_models um ON um.upstream_id = m.upstream_id AND um.model = m.upstream_model
                WHERE m.public_model = ? AND m.enabled = 1 AND u.enabled = 1 AND um.enabled = 1
                "#,
            )
            .bind(public_model)
            .fetch_all(&self.pool)
            .await?,
        );
        Ok(candidates)
    }

    pub async fn delete_api_key(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM api_keys WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn mark_api_key_used(&self, id: i64) -> Result<()> {
        sqlx::query("UPDATE api_keys SET last_used_at = ? WHERE id = ?")
            .bind(now())
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn insert_request_log(&self, input: LogInput) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO request_logs
            (api_key_id, public_model, upstream_id, upstream_model, status_code, input_tokens,
             output_tokens, total_tokens, error, duration_ms, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(input.api_key_id)
        .bind(input.public_model)
        .bind(input.upstream_id)
        .bind(input.upstream_model)
        .bind(i64::from(input.status_code))
        .bind(input.input_tokens)
        .bind(input.output_tokens)
        .bind(input.total_tokens)
        .bind(input.error)
        .bind(input.duration_ms)
        .bind(now())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_request_logs_paged(
        &self,
        params: &PageParams,
    ) -> Result<(Vec<RequestLog>, i64)> {
        let like = params.q.as_ref().map(|q| format!("%{q}%"));
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM request_logs l
            LEFT JOIN api_keys k ON k.id = l.api_key_id
            LEFT JOIN upstreams u ON u.id = l.upstream_id
            WHERE (? IS NULL
                   OR k.name LIKE ?
                   OR l.public_model LIKE ?
                   OR u.name LIKE ?
                   OR l.error LIKE ?)
            "#,
        )
        .bind(like.as_deref())
        .bind(like.as_deref())
        .bind(like.as_deref())
        .bind(like.as_deref())
        .bind(like.as_deref())
        .fetch_one(&self.pool)
        .await?;
        let rows = sqlx::query_as::<_, RequestLog>(
            r#"
            SELECT l.id, l.api_key_id, k.name AS api_key_name, l.public_model,
                   l.upstream_id, u.name AS upstream_name, l.upstream_model,
                   l.status_code, l.input_tokens, l.output_tokens, l.total_tokens,
                   l.error, l.duration_ms, l.created_at
            FROM request_logs l
            LEFT JOIN api_keys k ON k.id = l.api_key_id
            LEFT JOIN upstreams u ON u.id = l.upstream_id
            WHERE (? IS NULL
                   OR k.name LIKE ?
                   OR l.public_model LIKE ?
                   OR u.name LIKE ?
                   OR l.error LIKE ?)
            ORDER BY l.id DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(like.as_deref())
        .bind(like.as_deref())
        .bind(like.as_deref())
        .bind(like.as_deref())
        .bind(like.as_deref())
        .bind(params.page_size)
        .bind(params.offset())
        .fetch_all(&self.pool)
        .await?;
        Ok((rows, total))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn model_route_resolves_only_enabled_routes() {
        let db = Db::connect("sqlite::memory:").await.unwrap();
        let upstream = db
            .create_upstream(
                &UpstreamInput {
                    name: "test".into(),
                    base_url: "https://api.example.com/v1".into(),
                    api_key: String::new(),
                    enabled: true,
                    models: None,
                    model_configs: None,
                },
                "encrypted".into(),
            )
            .await
            .unwrap();

        db.create_model_route(&ModelRouteInput {
            public_model: "chat-main".into(),
            upstream_id: upstream.id,
            upstream_model: "real-model".into(),
            context_window: 128_000,
            max_context_window: 128_000,
            supports_parallel_tool_calls: true,
            supports_reasoning_summaries: false,
            supports_image_input: false,
            enabled: true,
        })
        .await
        .unwrap();
        db.upsert_upstream_models(
            upstream.id,
            &[UpstreamModelInput {
                model: "real-model".into(),
                enabled: true,
                context_window: 128_000,
                max_context_window: 128_000,
                supports_parallel_tool_calls: true,
                supports_reasoning_summaries: false,
                supports_image_input: false,
            }],
            None,
        )
        .await
        .unwrap();

        let routes = db.list_route_candidates(1, "chat-main").await.unwrap();
        assert_eq!(routes[0].upstream_model, "real-model");
        assert!(db
            .list_route_candidates(1, "missing")
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn upstream_model_inventory_preserves_disabled_state_on_sync() {
        let db = Db::connect("sqlite::memory:").await.unwrap();
        let upstream = db
            .create_upstream(
                &UpstreamInput {
                    name: "test".into(),
                    base_url: "https://api.example.com/v1".into(),
                    api_key: String::new(),
                    enabled: true,
                    models: None,
                    model_configs: None,
                },
                "encrypted".into(),
            )
            .await
            .unwrap();

        db.upsert_upstream_models(
            upstream.id,
            &[UpstreamModelInput {
                model: "real-model".into(),
                enabled: true,
                context_window: 128_000,
                max_context_window: 128_000,
                supports_parallel_tool_calls: true,
                supports_reasoning_summaries: false,
                supports_image_input: false,
            }],
            None,
        )
        .await
        .unwrap();
        db.save_upstream_models(
            upstream.id,
            &[UpstreamModelInput {
                model: "real-model".into(),
                enabled: false,
                context_window: 64_000,
                max_context_window: 96_000,
                supports_parallel_tool_calls: false,
                supports_reasoning_summaries: true,
                supports_image_input: true,
            }],
        )
        .await
        .unwrap();
        db.sync_upstream_model_inventory(
            upstream.id,
            &[UpstreamModelInput {
                model: "real-model".into(),
                enabled: true,
                context_window: 32_000,
                max_context_window: 48_000,
                supports_parallel_tool_calls: true,
                supports_reasoning_summaries: false,
                supports_image_input: false,
            }],
        )
        .await
        .unwrap();

        let model = db
            .list_local_upstream_models(upstream.id)
            .await
            .unwrap()
            .remove(0);
        assert!(!model.enabled);
        assert_eq!(model.context_window, 32_000);
        assert_eq!(model.max_context_window, 48_000);
        assert!(model.supports_parallel_tool_calls);
        assert!(!model.supports_reasoning_summaries);
        assert!(!model.supports_image_input);
    }

    #[tokio::test]
    async fn model_route_inherits_upstream_model_metadata() {
        let db = Db::connect("sqlite::memory:").await.unwrap();
        let upstream = db
            .create_upstream(
                &UpstreamInput {
                    name: "test".into(),
                    base_url: "https://api.example.com/v1".into(),
                    api_key: String::new(),
                    enabled: true,
                    models: None,
                    model_configs: None,
                },
                "encrypted".into(),
            )
            .await
            .unwrap();
        db.upsert_upstream_models(
            upstream.id,
            &[UpstreamModelInput {
                model: "real-model".into(),
                enabled: true,
                context_window: 64_000,
                max_context_window: 96_000,
                supports_parallel_tool_calls: false,
                supports_reasoning_summaries: true,
                supports_image_input: true,
            }],
            None,
        )
        .await
        .unwrap();

        let route = db
            .create_model_route(&ModelRouteInput {
                public_model: "chat-main".into(),
                upstream_id: upstream.id,
                upstream_model: "real-model".into(),
                context_window: 128_000,
                max_context_window: 128_000,
                supports_parallel_tool_calls: true,
                supports_reasoning_summaries: false,
                supports_image_input: false,
                enabled: true,
            })
            .await
            .unwrap();

        assert_eq!(route.context_window, 64_000);
        assert_eq!(route.max_context_window, 96_000);
        assert!(!route.supports_parallel_tool_calls);
        assert!(route.supports_reasoning_summaries);
        assert!(route.supports_image_input);
    }

    #[tokio::test]
    async fn api_key_can_be_disabled() {
        let db = Db::connect("sqlite::memory:").await.unwrap();
        let key = db
            .create_api_key(
                &ApiKeyInput {
                    name: "client".into(),
                    enabled: true,
                    models: None,
                },
                "hash",
                "encrypted-key".into(),
            )
            .await
            .unwrap();
        assert!(
            db.find_api_key_by_hash("hash")
                .await
                .unwrap()
                .unwrap()
                .enabled
        );
        db.set_api_key_enabled(key.id, false).await.unwrap();
        assert!(
            !db.find_api_key_by_hash("hash")
                .await
                .unwrap()
                .unwrap()
                .enabled
        );
    }

    #[tokio::test]
    async fn paged_upstreams_clamps_page_size_and_searches() {
        let db = Db::connect("sqlite::memory:").await.unwrap();
        for name in ["alpha", "beta", "gamma"] {
            db.create_upstream(
                &UpstreamInput {
                    name: name.into(),
                    base_url: format!("https://{name}.example.com/v1"),
                    api_key: String::new(),
                    enabled: true,
                    models: None,
                    model_configs: None,
                },
                format!("encrypted-{name}"),
            )
            .await
            .unwrap();
        }

        let (rows, total) = db
            .list_upstreams_paged(&PageParams::new(1, 2, None))
            .await
            .unwrap();
        assert_eq!(total, 3);
        assert_eq!(rows.len(), 2);

        let (rows, total) = db
            .list_upstreams_paged(&PageParams::new(1, 500, Some("alp".into())))
            .await
            .unwrap();
        assert_eq!(total, 1);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "alpha");
    }

    #[tokio::test]
    async fn paged_api_keys_preserves_model_authorization() {
        let db = Db::connect("sqlite::memory:").await.unwrap();
        for (name, models) in [
            ("client-a", Some(vec!["model-a".to_string()])),
            ("client-b", Some(vec!["model-b".to_string()])),
        ] {
            db.create_api_key(
                &ApiKeyInput {
                    name: name.into(),
                    enabled: true,
                    models,
                },
                &format!("hash-{name}"),
                format!("encrypted-{name}"),
            )
            .await
            .unwrap();
        }

        let (rows, total) = db
            .list_api_keys_paged(&PageParams::new(1, 1, Some("client-b".into())))
            .await
            .unwrap();
        assert_eq!(total, 1);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0.name, "client-b");
        assert_eq!(rows[0].1, vec!["model-b".to_string()]);
    }

    #[tokio::test]
    async fn paged_logs_are_recent_first_and_searchable() {
        let db = Db::connect("sqlite::memory:").await.unwrap();
        let upstream = db
            .create_upstream(
                &UpstreamInput {
                    name: "provider".into(),
                    base_url: "https://api.example.com/v1".into(),
                    api_key: String::new(),
                    enabled: true,
                    models: None,
                    model_configs: None,
                },
                "encrypted".into(),
            )
            .await
            .unwrap();

        db.insert_request_log(LogInput {
            api_key_id: None,
            public_model: Some("alpha".into()),
            upstream_id: Some(upstream.id),
            upstream_model: Some("real-alpha".into()),
            status_code: 200,
            input_tokens: 1,
            output_tokens: 1,
            total_tokens: 2,
            error: None,
            duration_ms: 10,
        })
        .await
        .unwrap();
        db.insert_request_log(LogInput {
            api_key_id: None,
            public_model: Some("beta".into()),
            upstream_id: Some(upstream.id),
            upstream_model: Some("real-beta".into()),
            status_code: 500,
            input_tokens: 1,
            output_tokens: 1,
            total_tokens: 2,
            error: Some("provider timeout".into()),
            duration_ms: 20,
        })
        .await
        .unwrap();

        let (rows, total) = db
            .list_request_logs_paged(&PageParams::new(1, 10, None))
            .await
            .unwrap();
        assert_eq!(total, 2);
        assert_eq!(rows[0].public_model.as_deref(), Some("beta"));

        let (rows, total) = db
            .list_request_logs_paged(&PageParams::new(1, 10, Some("timeout".into())))
            .await
            .unwrap();
        assert_eq!(total, 1);
        assert_eq!(rows[0].status_code, 500);
    }

    #[tokio::test]
    async fn app_settings_default_to_logging_disabled_and_persist() {
        let db = Db::connect("sqlite::memory:").await.unwrap();
        assert_eq!(db.get_app_settings().await.unwrap(), AppSettings::default());

        let saved = db
            .save_app_settings(&AppSettings {
                request_logging_enabled: true,
                upstream_timeout_seconds: 30,
                log_error_max_chars: 1200,
            })
            .await
            .unwrap();
        assert!(saved.request_logging_enabled);
        assert_eq!(saved.upstream_timeout_seconds, 30);
        assert_eq!(saved.log_error_max_chars, 1200);

        let loaded = db.get_app_settings().await.unwrap();
        assert_eq!(loaded, saved);
    }
}
