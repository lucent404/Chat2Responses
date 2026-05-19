use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    FromRow, SqlitePool,
};
use std::str::FromStr;

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
    pub created_at: String,
    pub updated_at: String,
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
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ApiKeyRecord {
    pub id: i64,
    pub name: String,
    #[serde(skip_serializing)]
    pub key_hash: String,
    pub enabled: bool,
    pub created_at: String,
    pub last_used_at: Option<String>,
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

#[derive(Debug, Deserialize)]
pub struct UpstreamInput {
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
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
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct ApiKeyInput {
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
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

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
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
                public_model TEXT NOT NULL UNIQUE,
                upstream_id INTEGER NOT NULL,
                upstream_model TEXT NOT NULL,
                context_window INTEGER NOT NULL DEFAULT 128000,
                max_context_window INTEGER NOT NULL DEFAULT 128000,
                supports_parallel_tool_calls INTEGER NOT NULL DEFAULT 1,
                supports_reasoning_summaries INTEGER NOT NULL DEFAULT 0,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY(upstream_id) REFERENCES upstreams(id) ON DELETE CASCADE
            )
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS api_keys (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                key_hash TEXT NOT NULL UNIQUE,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                last_used_at TEXT
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

    pub async fn list_upstreams(&self) -> Result<Vec<Upstream>> {
        Ok(sqlx::query_as::<_, Upstream>(
            "SELECT id, name, base_url, encrypted_api_key, enabled != 0 AS enabled, created_at, updated_at FROM upstreams ORDER BY id DESC",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn get_upstream(&self, id: i64) -> Result<Option<Upstream>> {
        Ok(sqlx::query_as::<_, Upstream>(
            "SELECT id, name, base_url, encrypted_api_key, enabled != 0 AS enabled, created_at, updated_at FROM upstreams WHERE id = ?",
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

    pub async fn list_model_routes(&self, enabled_only: bool) -> Result<Vec<ModelRoute>> {
        let sql = if enabled_only {
            r#"
            SELECT m.id, m.public_model, m.upstream_id, u.name AS upstream_name, m.upstream_model,
                   m.context_window, m.max_context_window,
                   m.supports_parallel_tool_calls != 0 AS supports_parallel_tool_calls,
                   m.supports_reasoning_summaries != 0 AS supports_reasoning_summaries,
                   m.enabled != 0 AS enabled, m.created_at, m.updated_at
            FROM model_routes m JOIN upstreams u ON u.id = m.upstream_id
            WHERE m.enabled = 1 AND u.enabled = 1
            ORDER BY m.public_model
            "#
        } else {
            r#"
            SELECT m.id, m.public_model, m.upstream_id, u.name AS upstream_name, m.upstream_model,
                   m.context_window, m.max_context_window,
                   m.supports_parallel_tool_calls != 0 AS supports_parallel_tool_calls,
                   m.supports_reasoning_summaries != 0 AS supports_reasoning_summaries,
                   m.enabled != 0 AS enabled, m.created_at, m.updated_at
            FROM model_routes m JOIN upstreams u ON u.id = m.upstream_id
            ORDER BY m.public_model
            "#
        };
        Ok(sqlx::query_as::<_, ModelRoute>(sql)
            .fetch_all(&self.pool)
            .await?)
    }

    pub async fn find_model_route(&self, public_model: &str) -> Result<Option<ModelRoute>> {
        Ok(sqlx::query_as::<_, ModelRoute>(
            r#"
            SELECT m.id, m.public_model, m.upstream_id, u.name AS upstream_name, m.upstream_model,
                   m.context_window, m.max_context_window,
                   m.supports_parallel_tool_calls != 0 AS supports_parallel_tool_calls,
                   m.supports_reasoning_summaries != 0 AS supports_reasoning_summaries,
                   m.enabled != 0 AS enabled, m.created_at, m.updated_at
            FROM model_routes m JOIN upstreams u ON u.id = m.upstream_id
            WHERE m.public_model = ? AND m.enabled = 1 AND u.enabled = 1
            "#,
        )
        .bind(public_model)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn create_model_route(&self, input: &ModelRouteInput) -> Result<ModelRoute> {
        let ts = now();
        let id = sqlx::query(
            r#"
            INSERT INTO model_routes
            (public_model, upstream_id, upstream_model, context_window, max_context_window,
             supports_parallel_tool_calls, supports_reasoning_summaries, enabled, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(input.public_model.trim())
        .bind(input.upstream_id)
        .bind(input.upstream_model.trim())
        .bind(input.context_window)
        .bind(input.max_context_window)
        .bind(input.supports_parallel_tool_calls)
        .bind(input.supports_reasoning_summaries)
        .bind(input.enabled)
        .bind(&ts)
        .bind(&ts)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();
        self.get_model_route(id).await?.ok_or_else(|| anyhow::anyhow!("created model route missing"))
    }

    pub async fn get_model_route(&self, id: i64) -> Result<Option<ModelRoute>> {
        Ok(sqlx::query_as::<_, ModelRoute>(
            r#"
            SELECT m.id, m.public_model, m.upstream_id, u.name AS upstream_name, m.upstream_model,
                   m.context_window, m.max_context_window,
                   m.supports_parallel_tool_calls != 0 AS supports_parallel_tool_calls,
                   m.supports_reasoning_summaries != 0 AS supports_reasoning_summaries,
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
                supports_reasoning_summaries = ?, enabled = ?, updated_at = ?
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

    pub async fn create_api_key(&self, input: &ApiKeyInput, key_hash: &str) -> Result<ApiKeyRecord> {
        let ts = now();
        let id = sqlx::query(
            "INSERT INTO api_keys (name, key_hash, enabled, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(input.name.trim())
        .bind(key_hash)
        .bind(input.enabled)
        .bind(&ts)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();
        Ok(ApiKeyRecord {
            id,
            name: input.name.trim().to_string(),
            key_hash: key_hash.to_string(),
            enabled: input.enabled,
            created_at: ts,
            last_used_at: None,
        })
    }

    pub async fn list_api_keys(&self) -> Result<Vec<ApiKeyRecord>> {
        Ok(sqlx::query_as::<_, ApiKeyRecord>(
            "SELECT id, name, key_hash, enabled != 0 AS enabled, created_at, last_used_at FROM api_keys ORDER BY id DESC",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn find_api_key_by_hash(&self, key_hash: &str) -> Result<Option<ApiKeyRecord>> {
        Ok(sqlx::query_as::<_, ApiKeyRecord>(
            "SELECT id, name, key_hash, enabled != 0 AS enabled, created_at, last_used_at FROM api_keys WHERE key_hash = ?",
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

    pub async fn list_request_logs(&self, limit: i64) -> Result<Vec<RequestLog>> {
        Ok(sqlx::query_as::<_, RequestLog>(
            r#"
            SELECT l.id, l.api_key_id, k.name AS api_key_name, l.public_model,
                   l.upstream_id, u.name AS upstream_name, l.upstream_model,
                   l.status_code, l.input_tokens, l.output_tokens, l.total_tokens,
                   l.error, l.duration_ms, l.created_at
            FROM request_logs l
            LEFT JOIN api_keys k ON k.id = l.api_key_id
            LEFT JOIN upstreams u ON u.id = l.upstream_id
            ORDER BY l.id DESC
            LIMIT ?
            "#,
        )
        .bind(limit.clamp(1, 500))
        .fetch_all(&self.pool)
        .await?)
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
            enabled: true,
        })
        .await
        .unwrap();

        let route = db.find_model_route("chat-main").await.unwrap().unwrap();
        assert_eq!(route.upstream_model, "real-model");
        assert!(db.find_model_route("missing").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn api_key_can_be_disabled() {
        let db = Db::connect("sqlite::memory:").await.unwrap();
        let key = db
            .create_api_key(
                &ApiKeyInput {
                    name: "client".into(),
                    enabled: true,
                },
                "hash",
            )
            .await
            .unwrap();
        assert!(db.find_api_key_by_hash("hash").await.unwrap().unwrap().enabled);
        db.set_api_key_enabled(key.id, false).await.unwrap();
        assert!(!db.find_api_key_by_hash("hash").await.unwrap().unwrap().enabled);
    }
}
