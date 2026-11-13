//! HTTP/PostgreSQL fixture for direct feedback tests.
use super::{DIRECT_KEY, GATEWAY, READ};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use std::{net::SocketAddr, sync::Arc};
use uuid::Uuid;

pub(super) struct Service {
    pub(super) base: String,
    admin: DatabaseConnection,
    pub(super) db: DatabaseConnection,
    schema: String,
    server: tokio::task::JoinHandle<()>,
}

impl Service {
    /// The service, taking direct reports when `direct`, believing the
    /// loopback proxy's `X-Forwarded-For`.
    pub(super) async fn start(what: &str, direct: bool) -> Self {
        let url = std::env::var("DATABASE_URL")
            .ok()
            .filter(|u| !u.is_empty())
            .expect("DATABASE_URL is not set, and these tests are about PostgreSQL");
        let schema: String = format!("fb_{what}_{}", Uuid::now_v7().simple())
            .chars()
            .take(63)
            .collect();
        let admin = Database::connect(&url).await.expect("connecting");
        admin
            .execute_unprepared(&format!("CREATE SCHEMA \"{schema}\""))
            .await
            .expect("a schema");
        let sep = if url.contains('?') { '&' } else { '?' };
        let scoped = format!("{url}{sep}options=-c%20search_path%3D{schema}");
        let db = baylee_feedback::connect(&scoped, 2)
            .await
            .expect("migrating");
        let mut config =
            baylee_feedback::Config::new(&format!("eu={GATEWAY}"), Some(READ), None).unwrap();
        if direct {
            config = config.with_direct_key(DIRECT_KEY).unwrap();
        }
        let ui = baylee_feedback::ui::Ui::default()
            .with_trusted_proxies(vec!["127.0.0.1".parse().unwrap()]);
        let app = baylee_feedback::app_with(
            Arc::new(baylee_feedback::AppState {
                db: db.clone(),
                config,
            }),
            ui,
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let _ = axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await;
        });
        Self {
            base,
            admin,
            db,
            schema,
            server,
        }
    }

    pub(super) async fn close(self) {
        self.server.abort();
        drop(self.db);
        self.admin
            .execute_unprepared(&format!("DROP SCHEMA \"{}\" CASCADE", self.schema))
            .await
            .expect("dropping the schema");
    }

    /// One request, as a client at `from` behind the proxy; the status, the
    /// `x-record-origin` header and the body.
    pub(super) async fn call(
        &self,
        method: &'static str,
        path: &str,
        token: Option<&str>,
        from: &str,
        body: Option<String>,
    ) -> (u16, Option<String>, serde_json::Value) {
        let url = format!("{}{path}", self.base);
        let token = token.map(str::to_owned);
        let from = from.to_owned();
        tokio::task::spawn_blocking(move || {
            let agent: ureq::Agent = ureq::Agent::config_builder()
                .http_status_as_error(false)
                .build()
                .into();
            let response = if method == "GET" {
                let mut r = agent.get(&url).header("X-Forwarded-For", &from);
                if let Some(t) = &token {
                    r = r.header("Authorization", format!("Bearer {t}"));
                }
                r.call()
            } else {
                let mut r = agent.post(&url).header("X-Forwarded-For", &from);
                if let Some(t) = &token {
                    r = r.header("Authorization", format!("Bearer {t}"));
                }
                r.header("Content-Type", "application/json")
                    .send(body.unwrap_or_default())
            };
            let mut response = response.expect("a response");
            let status = response.status().as_u16();
            let origin = response
                .headers()
                .get("x-record-origin")
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned);
            let bytes = response
                .body_mut()
                .with_config()
                .limit(64 * 1024 * 1024)
                .read_to_vec()
                .expect("a body");
            (
                status,
                origin,
                serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
            )
        })
        .await
        .unwrap()
    }

    pub(super) async fn direct(&self, from: &str, body: &serde_json::Value) -> u16 {
        self.call(
            "POST",
            "/client/reports",
            None,
            from,
            Some(body.to_string()),
        )
        .await
        .0
    }
}
