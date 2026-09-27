//! The service against a real PostgreSQL, over real HTTP.
//!
//! Each test runs in a schema of its own, created and dropped around it,
//! as the gateway's database tests do; without `DATABASE_URL` they fail
//! rather than skip.

use std::sync::Arc;

use base64::Engine as _;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};
use uuid::Uuid;

const GATEWAY: &str = "gateway-token-eu-0001";
const READ: &str = "read-token-000000001";
const ADMIN: &str = "admin-token-00000001";

struct Service {
    base: String,
    admin: DatabaseConnection,
    db: DatabaseConnection,
    schema: String,
}

impl Service {
    async fn start(what: &str) -> Self {
        let url = std::env::var("DATABASE_URL")
            .ok()
            .filter(|u| !u.is_empty())
            .expect(
                "DATABASE_URL is not set, and these tests are about PostgreSQL.\n  \
                 docker compose up -d\n  \
                 export DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee",
            );
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
            baylee_feedback::Config::new(&format!("eu={GATEWAY}"), Some(READ), Some(ADMIN))
                .expect("config");
        let app = baylee_feedback::app(Arc::new(baylee_feedback::AppState {
            db: db.clone(),
            config,
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await });
        Self {
            base,
            admin,
            db,
            schema,
        }
    }

    async fn close(self) {
        drop(self.db);
        self.admin
            .execute_unprepared(&format!("DROP SCHEMA \"{}\" CASCADE", self.schema))
            .await
            .expect("dropping the schema");
    }

    /// One request; the status and the body.
    async fn call(
        &self,
        method: &'static str,
        path: &str,
        token: Option<&str>,
        body: Option<String>,
    ) -> (u16, Vec<u8>) {
        let url = format!("{}{path}", self.base);
        let token = token.map(str::to_owned);
        tokio::task::spawn_blocking(move || {
            let agent: ureq::Agent = ureq::Agent::config_builder()
                .http_status_as_error(false)
                .build()
                .into();
            let auth = token.map(|t| format!("Bearer {t}"));
            let response = match (method, body) {
                ("GET", _) => {
                    let mut r = agent.get(&url);
                    if let Some(a) = &auth {
                        r = r.header("Authorization", a);
                    }
                    r.call()
                }
                ("DELETE", _) => {
                    let mut r = agent.delete(&url);
                    if let Some(a) = &auth {
                        r = r.header("Authorization", a);
                    }
                    r.call()
                }
                (method, body) => {
                    let mut r = if method == "PATCH" {
                        agent.patch(&url)
                    } else {
                        agent.post(&url)
                    };
                    if let Some(a) = &auth {
                        r = r.header("Authorization", a);
                    }
                    r.header("Content-Type", "application/json")
                        .send(body.unwrap_or_default())
                }
            };
            let mut response = response.expect("a response");
            let status = response.status().as_u16();
            let body = response
                .body_mut()
                .with_config()
                .limit(64 * 1024 * 1024)
                .read_to_vec()
                .expect("a body");
            (status, body)
        })
        .await
        .unwrap()
    }

    async fn json(
        &self,
        method: &'static str,
        path: &str,
        token: Option<&str>,
        body: Option<String>,
    ) -> (u16, serde_json::Value) {
        let (status, body) = self.call(method, path, token, body).await;
        (
            status,
            serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null),
        )
    }
}

fn a_report(kind: &str, record: Option<&[u8]>) -> String {
    serde_json::json!({
        "gateway": { "name": "Baylee EU", "url": "https://eu.example", "version": "0.1.0" },
        "reporter": "ab12cd34",
        "kind": kind,
        "text": "the swamp did a thing",
        "game_id": "g1",
        "client": { "os": "somewhere", "log": ["a", "b"] },
        "record": record.map(|r| serde_json::json!({
            "complete": true,
            "gzip_base64": base64::engine::general_purpose::STANDARD.encode(r),
        })),
    })
    .to_string()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::too_many_lines)] // one scenario, intake to read-back
async fn a_gateway_hands_in_a_report_and_a_reader_reads_it_back() {
    let service = Service::start("intake").await;
    let record = b"\x1f\x8bnot really gzip but bytes".to_vec();

    assert_eq!(
        service
            .call("POST", "/intake/reports", None, Some(a_report("bug", None)))
            .await
            .0,
        401
    );
    assert_eq!(
        service
            .call(
                "POST",
                "/intake/reports",
                Some(READ),
                Some(a_report("bug", None))
            )
            .await
            .0,
        401,
        "a read token hands in nothing"
    );
    let (status, made) = service
        .json(
            "POST",
            "/intake/reports",
            Some(GATEWAY),
            Some(a_report("bug", Some(&record))),
        )
        .await;
    assert_eq!(status, 201);
    let id = made["report_id"].as_str().expect("an id").to_owned();
    assert_eq!(Uuid::parse_str(&id).unwrap().get_version_num(), 7);
    let (status, _) = service
        .json(
            "POST",
            "/intake/reports",
            Some(GATEWAY),
            Some(a_report("feedback", None)),
        )
        .await;
    assert_eq!(status, 201);

    assert_eq!(service.call("GET", "/reports", None, None).await.0, 401);
    assert_eq!(
        service.call("GET", "/reports", Some(GATEWAY), None).await.0,
        401,
        "a gateway reads nothing"
    );
    let (status, all) = service.json("GET", "/reports", Some(READ), None).await;
    assert_eq!(status, 200);
    assert_eq!(all["total"], 2);
    let (_, bugs) = service
        .json("GET", "/reports?kind=bug", Some(READ), None)
        .await;
    assert_eq!(bugs["total"], 1);
    let bug = &bugs["reports"][0];
    assert_eq!(bug["id"], id.as_str());
    assert_eq!(
        bug["gateway"], "eu",
        "the token's name, not the gateway's claim"
    );
    assert_eq!(bug["gateway_name"], "Baylee EU");
    assert_eq!(bug["status"], "new");
    assert_eq!(bug["has_record"], true);
    assert_eq!(bug["record_bytes"], record.len());
    assert!(
        bug.get("client").is_none(),
        "the list leaves the client out"
    );
    assert_eq!(
        service
            .json("GET", "/reports?kind=nonsense", Some(READ), None)
            .await
            .0,
        400
    );
    assert_eq!(
        service
            .json("GET", "/reports?gateway=elsewhere", Some(READ), None)
            .await
            .1["total"],
        0
    );

    let (status, one) = service
        .json("GET", &format!("/reports/{id}"), Some(READ), None)
        .await;
    assert_eq!(status, 200);
    assert_eq!(one["client"]["os"], "somewhere");
    let (status, bytes) = service
        .call("GET", &format!("/reports/{id}/record"), Some(READ), None)
        .await;
    assert_eq!(status, 200);
    assert_eq!(bytes, record);
    assert_eq!(
        service
            .call("GET", "/reports/not-an-id", Some(READ), None)
            .await
            .0,
        404
    );
    assert_eq!(
        service
            .call(
                "GET",
                &format!("/reports/{}", Uuid::now_v7()),
                Some(READ),
                None
            )
            .await
            .0,
        404
    );

    service.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn only_the_admin_token_changes_or_deletes_a_report() {
    let service = Service::start("admin").await;
    let (_, made) = service
        .json(
            "POST",
            "/intake/reports",
            Some(GATEWAY),
            Some(a_report("crash", None)),
        )
        .await;
    let id = made["report_id"].as_str().unwrap().to_owned();
    let path = format!("/reports/{id}");
    let triaged = Some(r#"{"status":"triaged"}"#.to_owned());

    assert_eq!(
        service
            .call("PATCH", &path, Some(READ), triaged.clone())
            .await
            .0,
        401
    );
    assert_eq!(
        service
            .call(
                "PATCH",
                &path,
                Some(ADMIN),
                Some(r#"{"status":"lost"}"#.into())
            )
            .await
            .0,
        400
    );
    let (status, changed) = service.json("PATCH", &path, Some(ADMIN), triaged).await;
    assert_eq!(status, 200);
    assert_eq!(changed["status"], "triaged");
    let (_, listed) = service
        .json("GET", "/reports?status=triaged", Some(ADMIN), None)
        .await;
    assert_eq!(listed["total"], 1, "the admin token also reads");

    assert_eq!(service.call("DELETE", &path, Some(READ), None).await.0, 401);
    assert_eq!(
        service.call("DELETE", &path, Some(ADMIN), None).await.0,
        204
    );
    assert_eq!(
        service.call("DELETE", &path, Some(ADMIN), None).await.0,
        404
    );
    assert_eq!(service.call("GET", &path, Some(READ), None).await.0, 404);

    service.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_malformed_report_is_refused_and_nothing_about_the_caller_is_kept() {
    let service = Service::start("refused").await;
    let mut bad: serde_json::Value = serde_json::from_str(&a_report("bug", None)).unwrap();
    bad["client"] = serde_json::json!("not an object");
    for body in [
        bad.to_string(),
        a_report("rant", None),
        "{}".to_owned(),
        a_report("bug", None).replace("ab12cd34", "Real Name"),
    ] {
        assert_eq!(
            service
                .call("POST", "/intake/reports", Some(GATEWAY), Some(body.clone()))
                .await
                .0,
            400,
            "{body}"
        );
    }
    let mut long: serde_json::Value = serde_json::from_str(&a_report("bug", None)).unwrap();
    long["text"] = serde_json::json!("x".repeat(20_001));
    assert_eq!(
        service
            .call(
                "POST",
                "/intake/reports",
                Some(GATEWAY),
                Some(long.to_string())
            )
            .await
            .0,
        413
    );

    // The table has no column a request's address or a person's name could
    // go into: the whole list, so a new one has to be added here on purpose.
    let columns: Vec<String> = service
        .db
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT column_name::text AS c FROM information_schema.columns \
             WHERE table_schema = $1 AND table_name = 'feedback_report' ORDER BY ordinal_position",
            [service.schema.clone().into()],
        ))
        .await
        .unwrap()
        .iter()
        .map(|r| r.try_get::<String>("", "c").unwrap())
        .collect();
    assert_eq!(
        columns,
        [
            "id",
            "created_at",
            "updated_at",
            "gateway",
            "gateway_name",
            "gateway_url",
            "gateway_version",
            "reporter",
            "kind",
            "status",
            "text",
            "game_id",
            "client",
            "record",
            "record_complete",
        ]
    );

    service.close().await;
}
