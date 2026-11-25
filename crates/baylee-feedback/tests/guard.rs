//! The guards in front of the token routes (`guard.rs`, owner 25.11.2026:
//! the intake stays public, #408), against a real PostgreSQL over real
//! HTTP, behind a loopback "proxy" whose `X-Forwarded-For` the service
//! believes. Without `DATABASE_URL` these fail rather than skip.

use std::{net::SocketAddr, sync::Arc};

use base64::Engine as _;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use uuid::Uuid;

const EU: &str = "gateway-token-eu-0001";
const US: &str = "gateway-token-us-0002";
const READ: &str = "read-token-000000001";
const ADMIN: &str = "admin-token-00000001";

struct Service {
    base: String,
    admin: DatabaseConnection,
    db: DatabaseConnection,
    schema: String,
    server: tokio::task::JoinHandle<()>,
}

impl Service {
    async fn start(what: &str, ui: baylee_feedback::ui::Ui) -> Self {
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
        let config =
            baylee_feedback::Config::new(&format!("eu={EU},us={US}"), Some(READ), Some(ADMIN))
                .unwrap();
        let app = baylee_feedback::app_with(
            Arc::new(baylee_feedback::AppState {
                db: db.clone(),
                config,
            }),
            ui.with_trusted_proxies(vec!["127.0.0.1".parse().unwrap()]),
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

    async fn close(self) {
        self.server.abort();
        drop(self.db);
        self.admin
            .execute_unprepared(&format!("DROP SCHEMA \"{}\" CASCADE", self.schema))
            .await
            .expect("dropping the schema");
    }

    /// One request with `token`, from `from` behind the proxy, or from this
    /// machine without it when `from` is `None`; the status.
    async fn call(
        &self,
        method: &'static str,
        path: &str,
        token: &str,
        from: Option<&str>,
        body: Option<String>,
    ) -> u16 {
        let url = format!("{}{path}", self.base);
        let auth = format!("Bearer {token}");
        let from = from.map(str::to_owned);
        tokio::task::spawn_blocking(move || {
            let agent: ureq::Agent = ureq::Agent::config_builder()
                .http_status_as_error(false)
                .build()
                .into();
            let response = if method == "GET" {
                let mut r = agent.get(&url).header("Authorization", &auth);
                if let Some(from) = &from {
                    r = r.header("X-Forwarded-For", from);
                }
                r.call()
            } else {
                let mut r = agent.post(&url).header("Authorization", &auth);
                if let Some(from) = &from {
                    r = r.header("X-Forwarded-For", from);
                }
                r.header("Content-Type", "application/json")
                    .send(body.unwrap_or_default())
            };
            response.expect("a response").status().as_u16()
        })
        .await
        .unwrap()
    }

    async fn file(&self, token: &str, from: Option<&str>, record: Option<&[u8]>) -> u16 {
        self.call(
            "POST",
            "/intake/reports",
            token,
            from,
            Some(a_report(record)),
        )
        .await
    }

    async fn stored(&self) -> i64 {
        let row = self
            .db
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DbBackend::Postgres,
                "SELECT count(*) AS n FROM feedback_report",
            ))
            .await
            .unwrap()
            .unwrap();
        row.try_get("", "n").unwrap()
    }
}

fn a_report(record: Option<&[u8]>) -> String {
    serde_json::json!({
        "gateway": { "name": "Collector", "url": null, "version": "0.1.0" },
        "reporter": "ab12cd34",
        "kind": "bug",
        "text": "the model says the swamp did a thing",
        "game_id": "g1",
        "client": { "source": "llm-seat" },
        "record": record.map(|r| serde_json::json!({
            "complete": false,
            "gzip_base64": base64::engine::general_purpose::STANDARD.encode(r),
        })),
    })
    .to_string()
}

/// Twenty wrong credentials from one address shut it out of every token
/// route for a quarter of an hour, its right tokens too; other addresses,
/// and a process on this machine that does not come through the proxy, are
/// heard as before.
#[tokio::test]
async fn wrong_credentials_shut_their_address_and_no_one_else() {
    let service = Service::start("guard_shut", baylee_feedback::ui::Ui::default()).await;
    let guesser = Some("203.0.113.7");
    for n in 0..baylee_feedback::guard::FAILURES_PER_ADDRESS {
        let wrong = format!("gateway-token-guess-{n:04}");
        assert_eq!(service.file(&wrong, guesser, None).await, 401, "guess {n}");
    }
    // shut: the right token is not even looked at, on any token route
    assert_eq!(service.file(EU, guesser, None).await, 429);
    assert_eq!(
        service.call("GET", "/reports", READ, guesser, None).await,
        429
    );
    // the same /64 is the same guesser; IPv4 and IPv6 alike
    let v6 = Some("2001:db8:7:7::1");
    for n in 0..baylee_feedback::guard::FAILURES_PER_ADDRESS {
        let status = service
            .call("GET", "/reports", &format!("read-guess-{n:010}"), v6, None)
            .await;
        assert_eq!(status, 401);
    }
    assert_eq!(
        service
            .call("GET", "/reports", READ, Some("2001:db8:7:7:ffff::9"), None)
            .await,
        429
    );
    // everyone else is heard
    assert_eq!(service.file(EU, Some("203.0.113.8"), None).await, 201);
    assert_eq!(
        service
            .call("GET", "/reports", READ, Some("2001:db8:7:8::1"), None)
            .await,
        200
    );
    // the official gateway on this machine: its mistakes are not counted,
    // and nobody's guesses shut it out
    for n in 0..30 {
        let wrong = format!("gateway-token-typo-{n:04}");
        assert_eq!(service.file(&wrong, None, None).await, 401);
    }
    assert_eq!(service.file(EU, None, None).await, 201);
    assert_eq!(service.stored().await, 2);
    service.close().await;
}

/// A gateway's token files at most its reports an hour, and each gateway
/// has its own.
#[tokio::test]
async fn a_gateway_s_token_files_its_reports_an_hour_and_no_more() {
    let ui = baylee_feedback::ui::Ui::default().with_intake_budget(2, 1024);
    let service = Service::start("guard_count", ui).await;
    let from = Some("198.51.100.20");
    assert_eq!(service.file(EU, from, None).await, 201);
    assert_eq!(service.file(EU, from, None).await, 201);
    assert_eq!(service.file(EU, from, None).await, 429, "two an hour");
    assert_eq!(service.file(US, from, None).await, 201, "us has its own");
    assert_eq!(service.stored().await, 3);
    // reading is not filing
    assert_eq!(service.call("GET", "/reports", READ, from, None).await, 200);
    service.close().await;
}

/// The bytes are budgeted too: one large report may spend a gateway's
/// hour however few reports it sent.
#[tokio::test]
async fn a_gateway_s_token_sends_its_bytes_an_hour_and_no_more() {
    let ui = baylee_feedback::ui::Ui::default().with_intake_budget(100, 1);
    let service = Service::start("guard_bytes", ui).await;
    // a gzip stream's magic and 1.1 MiB after it, base64 in the body
    let mut record = vec![0x1f, 0x8b];
    record.resize(1_100_000, 7);
    let from = Some("198.51.100.21");
    assert_eq!(service.file(EU, from, Some(&record)).await, 201);
    assert_eq!(service.file(EU, from, None).await, 429, "the MiB is spent");
    assert_eq!(service.file(US, from, None).await, 201);
    service.close().await;
}
