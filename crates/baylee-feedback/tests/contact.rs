//! `POST /site/contact` and the web UI's routes for the messages it keeps
//! (`docs/feedback.md` §"The landing page's contact form") against a real
//! PostgreSQL, over real HTTP: the cross-origin rules, the limits, what is
//! stored, the allowance per address, the admin's list, status and delete
//! with their audit, and the retention sweep.
//!
//! Each test runs in a schema of its own, as `direct.rs` and `ui.rs` do;
//! without `DATABASE_URL` they fail rather than skip.

use std::net::SocketAddr;
use std::sync::Arc;

use baylee_feedback::ui::{self, Ui};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};
use uuid::Uuid;

const GATEWAY: &str = "gateway-token-eu-0001";
const READ: &str = "read-token-000000001";
const ADMIN_TOKEN: &str = "admin-token-00000001";
const CONSOLE_TOKEN: &str = "the-gateways-admin-token-0123456789abcdef";

const NAME: &str = "viktor";
const PASSWORD: &str = "a long enough password";

/// The page the form is served from.
const SITE: &str = "https://site.example";
const EVIL: &str = "https://evil.example";

struct Answer {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Answer {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
    fn json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body).unwrap_or(serde_json::Value::Null)
    }
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
    fn session_cookie(&self) -> Option<String> {
        let set = self.header("set-cookie")?;
        let pair = set.split(';').next()?.trim();
        pair.starts_with(ui::COOKIE).then(|| pair.to_owned())
    }
}

struct Service {
    base: String,
    admin: DatabaseConnection,
    db: DatabaseConnection,
    schema: String,
    server: tokio::task::JoinHandle<()>,
}

impl Service {
    /// The service taking contact messages from `origins` (none = off),
    /// believing the loopback proxy's `X-Forwarded-For`, with one admin.
    async fn start(what: &str, origins: Option<&str>) -> Self {
        let url = std::env::var("DATABASE_URL")
            .ok()
            .filter(|u| !u.is_empty())
            .expect(
                "DATABASE_URL is not set, and these tests are about PostgreSQL.\n  \
                 docker compose up -d\n  \
                 export DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee",
            );
        let schema: String = format!("fbct_{what}_{}", Uuid::now_v7().simple())
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
        baylee_feedback::admin::add(&db, NAME, PASSWORD)
            .await
            .expect("an admin");
        let mut config =
            baylee_feedback::Config::new(&format!("eu={GATEWAY}"), Some(READ), Some(ADMIN_TOKEN))
                .and_then(|c| {
                    // Never dialled: the audit list asks only this service's
                    // database, but the console routes want it configured.
                    c.with_gateway_admin("http://127.0.0.1:1", CONSOLE_TOKEN)
                })
                .expect("config");
        if let Some(origins) = origins {
            config = config.with_contact_origins(origins).expect("origins");
        }
        let ui = Ui::default().with_trusted_proxies(vec!["127.0.0.1".parse().unwrap()]);
        let app = baylee_feedback::app_with(
            Arc::new(baylee_feedback::AppState {
                db: db.clone(),
                config,
            }),
            ui,
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let _ = axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await;
        });
        Self {
            base: format!("http://{addr}"),
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

    /// One request with exactly these headers.
    async fn call(
        &self,
        method: &str,
        path: &str,
        headers: &[(&str, String)],
        body: Option<String>,
    ) -> Answer {
        let url = format!("{}{path}", self.base);
        let method = method.to_owned();
        let headers: Vec<(String, String)> = headers
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect();
        tokio::task::spawn_blocking(move || {
            let agent: ureq::Agent = ureq::Agent::config_builder()
                .http_status_as_error(false)
                .allow_non_standard_methods(true)
                .build()
                .into();
            let mut request = ureq::http::Request::builder()
                .method(method.as_str())
                .uri(&url);
            for (k, v) in &headers {
                request = request.header(k, v);
            }
            if body.is_some() {
                request = request.header("content-type", "application/json");
            }
            let request = request.body(body.unwrap_or_default().into_bytes()).unwrap();
            let mut response = agent.run(request).expect("a response");
            let status = response.status().as_u16();
            let headers = response
                .headers()
                .iter()
                .map(|(k, v)| (k.as_str().to_owned(), v.to_str().unwrap_or("").to_owned()))
                .collect();
            let body = response
                .body_mut()
                .with_config()
                .limit(64 * 1024 * 1024)
                .read_to_vec()
                .unwrap_or_default();
            Answer {
                status,
                headers,
                body,
            }
        })
        .await
        .unwrap()
    }

    /// The page at `origin` posts `body` as a client at `from`.
    async fn post_as(&self, origin: Option<&str>, from: &str, body: String) -> Answer {
        let mut headers = vec![("x-forwarded-for", from.to_owned())];
        if let Some(origin) = origin {
            headers.push(("origin", origin.to_owned()));
        }
        self.call("POST", "/site/contact", &headers, Some(body))
            .await
    }

    /// The allowed page posts a message built from `fields` over a good one.
    async fn send(&self, from: &str, fields: &serde_json::Value) -> Answer {
        let mut body = message();
        for (k, v) in fields.as_object().unwrap() {
            body[k] = v.clone();
        }
        self.post_as(Some(SITE), from, body.to_string()).await
    }

    /// The headers this UI's own pages send with a change.
    fn same_origin(&self) -> Vec<(&'static str, String)> {
        vec![
            ("origin", self.base.clone()),
            (ui::CSRF_HEADER, "1".to_owned()),
        ]
    }

    /// As [`Self::same_origin`], signed in with `cookie`.
    fn signed_in(&self, cookie: &str) -> Vec<(&'static str, String)> {
        let mut headers = self.same_origin();
        headers.push(("cookie", cookie.to_owned()));
        headers
    }

    /// Signs in as the test's admin; the cookie to send.
    async fn login(&self) -> String {
        let answer = self
            .call(
                "POST",
                "/ui/api/login",
                &self.same_origin(),
                Some(serde_json::json!({ "name": NAME, "password": PASSWORD }).to_string()),
            )
            .await;
        assert_eq!(answer.status, 200, "{}", answer.text());
        answer.session_cookie().expect("a session cookie")
    }

    async fn count(&self, sql: &str) -> i64 {
        self.db
            .query_one_raw(Statement::from_string(DbBackend::Postgres, sql))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "n")
            .unwrap()
    }

    async fn stored(&self) -> i64 {
        self.count("SELECT count(*) AS n FROM feedback_contact")
            .await
    }

    /// Puts a message in by SQL; `created` and `done` are Postgres
    /// intervals back from now (`done` makes it a done message).
    async fn insert(&self, text: &str, created: &str, done: Option<&str>) -> Uuid {
        let id = Uuid::now_v7();
        let (status, done_at) = match done {
            Some(ago) => ("done", format!("now() - interval '{ago}'")),
            None => ("new", "NULL".to_owned()),
        };
        self.db
            .execute_unprepared(&format!(
                "INSERT INTO feedback_contact (id, created_at, message, language, status, done_at) \
                 VALUES ('{id}', now() - interval '{created}', '{text}', 'en', '{status}', {done_at})"
            ))
            .await
            .unwrap();
        id
    }

    /// The first message id the admin's list shows.
    async fn listed(&self, cookie: &str, query: &str) -> serde_json::Value {
        let answer = self
            .call(
                "GET",
                &format!("/ui/api/contact{query}"),
                &[("cookie", cookie.to_owned())],
                None,
            )
            .await;
        assert_eq!(answer.status, 200, "{}", answer.text());
        answer.json()
    }

    /// The audit rows of one message, oldest first: `actor/action/detail`.
    async fn audit_of(&self, id: &str) -> Vec<String> {
        self.db
            .query_all_raw(Statement::from_string(
                DbBackend::Postgres,
                format!(
                    "SELECT actor || '/' || action || '/' || coalesce(detail, '') AS line \
                     FROM feedback_audit WHERE contact_id = '{id}' ORDER BY at, id"
                ),
            ))
            .await
            .unwrap()
            .iter()
            .map(|row| row.try_get("", "line").unwrap())
            .collect()
    }
}

/// A good message from the German page.
fn message() -> serde_json::Value {
    serde_json::json!({
        "name": "Max Mustermann",
        "email": "max@example.org",
        "message": "Hallo, ein Spiel hat gehangen.",
        "language": "de",
    })
}

// ------------------------------------------------------------------ the form

/// A message from the allowed page is taken: `201 {"ok":true}`, the page
/// may read the answer, and the row holds what was typed, still `new`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_message_from_the_allowed_page_is_stored_and_the_page_may_read_the_answer() {
    let service = Service::start("valid", Some(SITE)).await;
    let answer = service
        .post_as(Some(SITE), "203.0.113.1", message().to_string())
        .await;
    assert_eq!(answer.status, 201, "{}", answer.text());
    assert_eq!(answer.json(), serde_json::json!({ "ok": true }));
    assert_eq!(answer.header("access-control-allow-origin"), Some(SITE));
    assert_eq!(answer.header("vary"), Some("Origin"));
    let row = service
        .db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT name, email, message, language, status, \
                    done_at IS NULL AS open FROM feedback_contact",
        ))
        .await
        .unwrap()
        .expect("a row");
    assert_eq!(row.try_get::<String>("", "name").unwrap(), "Max Mustermann");
    assert_eq!(
        row.try_get::<String>("", "email").unwrap(),
        "max@example.org"
    );
    assert_eq!(
        row.try_get::<String>("", "message").unwrap(),
        "Hallo, ein Spiel hat gehangen."
    );
    assert_eq!(row.try_get::<String>("", "language").unwrap(), "de");
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "new");
    assert!(row.try_get::<bool>("", "open").unwrap(), "not done yet");
    assert_eq!(service.stored().await, 1);
    service.close().await;
}

/// Name and e-mail are optional, and the page's `Origin` is matched
/// without regard to case but answered as configured.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_message_needs_only_text_and_language_and_the_origin_is_matched_in_any_case() {
    let service = Service::start("minimal", Some("https://Site.Example")).await;
    let answer = service
        .post_as(
            Some("HTTPS://SITE.EXAMPLE"),
            "203.0.113.1",
            serde_json::json!({ "message": "  just this  ", "language": "en" }).to_string(),
        )
        .await;
    assert_eq!(answer.status, 201, "{}", answer.text());
    let row = service
        .db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT name IS NULL AS no_name, email IS NULL AS no_email, message \
             FROM feedback_contact",
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(row.try_get::<bool>("", "no_name").unwrap());
    assert!(row.try_get::<bool>("", "no_email").unwrap());
    assert_eq!(row.try_get::<String>("", "message").unwrap(), "just this");
    service.close().await;
}

/// Without `FEEDBACK_CONTACT_ORIGINS` the route is off.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn without_origins_configured_the_contact_route_takes_nothing() {
    let service = Service::start("off", None).await;
    let answer = service
        .post_as(Some(SITE), "203.0.113.1", message().to_string())
        .await;
    assert_eq!(answer.status, 503, "{}", answer.text());
    assert_eq!(service.stored().await, 0);
    service.close().await;
}

/// No `Origin`, or another one, is refused, is not told it may read the
/// answer, and stores nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_request_from_another_origin_or_none_is_refused_without_a_cors_answer() {
    let service = Service::start("origin", Some(SITE)).await;
    for origin in [None, Some(EVIL), Some("null"), Some("http://site.example")] {
        let answer = service
            .post_as(origin, "203.0.113.1", message().to_string())
            .await;
        assert_eq!(answer.status, 403, "{origin:?}: {}", answer.text());
        assert_eq!(
            answer.header("access-control-allow-origin"),
            None,
            "{origin:?}"
        );
    }
    assert_eq!(service.stored().await, 0);
    // A refusal for its origin is not counted against the address.
    for _ in 0..6 {
        let answer = service
            .post_as(Some(EVIL), "203.0.113.2", message().to_string())
            .await;
        assert_eq!(answer.status, 403);
    }
    assert_eq!(
        service
            .send("203.0.113.2", &serde_json::json!({}))
            .await
            .status,
        201
    );
    service.close().await;
}

/// The browser's preflight is answered for the allowed page only.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_preflight_is_answered_for_the_allowed_origin_only() {
    let service = Service::start("preflight", Some(SITE)).await;
    let ask = |origin: &str| {
        vec![
            ("origin", origin.to_owned()),
            ("access-control-request-method", "POST".to_owned()),
            ("access-control-request-headers", "content-type".to_owned()),
        ]
    };
    let answer = service
        .call("OPTIONS", "/site/contact", &ask(SITE), None)
        .await;
    assert_eq!(answer.status, 204);
    assert_eq!(answer.header("access-control-allow-origin"), Some(SITE));
    assert!(
        answer
            .header("access-control-allow-methods")
            .is_some_and(|m| m.contains("POST")),
        "{:?}",
        answer.headers
    );
    assert!(
        answer
            .header("access-control-allow-headers")
            .is_some_and(|h| h.to_ascii_lowercase().contains("content-type")),
        "{:?}",
        answer.headers
    );
    assert_eq!(answer.header("vary"), Some("Origin"));

    let answer = service
        .call("OPTIONS", "/site/contact", &ask(EVIL), None)
        .await;
    assert_eq!(answer.status, 403);
    assert_eq!(answer.header("access-control-allow-origin"), None);
    let answer = service.call("OPTIONS", "/site/contact", &[], None).await;
    assert_eq!(answer.status, 403);
    assert_eq!(answer.header("access-control-allow-origin"), None);
    service.close().await;
}

/// A filled honeypot is answered like a message taken, and nothing is kept.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_filled_honeypot_is_answered_as_taken_and_nothing_is_stored() {
    let service = Service::start("honeypot", Some(SITE)).await;
    let answer = service
        .send(
            "203.0.113.1",
            &serde_json::json!({ "website": "http://spam" }),
        )
        .await;
    assert_eq!(answer.status, 201, "{}", answer.text());
    assert_eq!(answer.json(), serde_json::json!({ "ok": true }));
    assert_eq!(service.stored().await, 0);
    // An empty honeypot is what a person leaves.
    let answer = service
        .send("203.0.113.1", &serde_json::json!({ "website": "  " }))
        .await;
    assert_eq!(answer.status, 201);
    assert_eq!(service.stored().await, 1);
    service.close().await;
}

/// What is not a message is refused by its kind: unknown fields, an empty
/// text, a language the page has not, and a field over its bound.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_malformed_message_is_refused_and_nothing_is_stored() {
    let service = Service::start("refused", Some(SITE)).await;
    let from = "203.0.113.1";
    let unknown = service
        .send(from, &serde_json::json!({ "phone": "123" }))
        .await;
    assert_eq!(unknown.status, 400, "unknown field: {}", unknown.text());
    // Every case from an address of its own: refused ones count.
    for (n, (why, fields, want)) in [
        ("empty text", serde_json::json!({ "message": "" }), 400),
        (
            "blank text",
            serde_json::json!({ "message": " \n\t " }),
            400,
        ),
        ("bad language", serde_json::json!({ "language": "fr" }), 400),
        (
            "bad e-mail",
            serde_json::json!({ "email": "not an address" }),
            400,
        ),
        (
            "control character",
            serde_json::json!({ "message": "a\u{0}b" }),
            400,
        ),
        (
            "too long a message",
            serde_json::json!({ "message": "a".repeat(5_001) }),
            413,
        ),
        (
            "too long a name",
            serde_json::json!({ "name": "n".repeat(101) }),
            413,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let answer = service.send(&format!("198.51.100.{n}"), &fields).await;
        assert_eq!(answer.status, want, "{why}: {}", answer.text());
    }
    for (why, body) in [
        ("not json", "not json".to_owned()),
        ("no language", r#"{"message":"hi"}"#.to_owned()),
        ("no message", r#"{"language":"en"}"#.to_owned()),
    ] {
        let answer = service.post_as(Some(SITE), "203.0.113.9", body).await;
        assert_eq!(answer.status, 400, "{why}: {}", answer.text());
    }
    assert_eq!(service.stored().await, 0);
    // The longest message is taken, to the character.
    let answer = service
        .send(
            "203.0.113.8",
            &serde_json::json!({ "message": "ä".repeat(5_000) }),
        )
        .await;
    assert_eq!(answer.status, 201, "{}", answer.text());
    service.close().await;
}

/// A body past the route's limit is refused before it is read.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_body_past_the_limit_is_too_large() {
    let service = Service::start("body", Some(SITE)).await;
    let answer = service
        .send(
            "203.0.113.1",
            &serde_json::json!({ "message": "a".repeat(40 * 1024) }),
        )
        .await;
    assert_eq!(answer.status, 413, "{}", answer.text());
    assert_eq!(service.stored().await, 0);
    service.close().await;
}

/// Five an hour per address, counting the refused too, and the page can
/// still read the `429`; another address is not affected.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_sixth_message_from_one_address_is_too_many_and_another_address_is_not_affected() {
    let service = Service::start("allowance", Some(SITE)).await;
    let from = "203.0.113.50";
    for n in 0..3 {
        let answer = service.send(from, &serde_json::json!({})).await;
        assert_eq!(answer.status, 201, "message {n}");
    }
    // Refused for what they say, they still count.
    for n in 3..5 {
        let answer = service
            .send(from, &serde_json::json!({ "message": "" }))
            .await;
        assert_eq!(answer.status, 400, "message {n}");
    }
    let answer = service.send(from, &serde_json::json!({})).await;
    assert_eq!(answer.status, 429, "{}", answer.text());
    assert_eq!(
        answer.header("access-control-allow-origin"),
        Some(SITE),
        "the page can read why"
    );
    let answer = service
        .send(from, &serde_json::json!({ "message": "" }))
        .await;
    assert_eq!(
        answer.status, 429,
        "a refused one is not asked about its text"
    );
    assert_eq!(service.stored().await, 3, "only the good ones");

    let answer = service.send("203.0.113.51", &serde_json::json!({})).await;
    assert_eq!(answer.status, 201, "another address");
    service.close().await;
}

// ------------------------------------------------------------------ the UI

/// Every contact route wants a session.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_admin_routes_refuse_without_a_session() {
    let service = Service::start("nosession", Some(SITE)).await;
    let id = service.insert("hi", "1 day", None).await;
    let bogus = format!("{}={}", ui::COOKIE, "0".repeat(64));
    for (method, path, body) in [
        ("GET", "/ui/api/contact".to_owned(), None),
        ("GET", format!("/ui/api/contact/{id}"), None),
        (
            "PATCH",
            format!("/ui/api/contact/{id}"),
            Some(r#"{"status":"done"}"#.to_owned()),
        ),
        ("DELETE", format!("/ui/api/contact/{id}"), None),
    ] {
        for credential in [None, Some(("cookie", bogus.clone()))] {
            let mut headers = service.same_origin();
            headers.extend(credential);
            let answer = service.call(method, &path, &headers, body.clone()).await;
            assert_eq!(answer.status, 401, "{method} {path}");
        }
    }
    assert_eq!(service.stored().await, 1, "nothing was changed");
    service.close().await;
}

/// The list answers the newest first, how many are open and how many the
/// filter matches, and refuses a filter it does not know.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_list_is_newest_first_counts_the_open_ones_and_filters_by_status() {
    let service = Service::start("list", Some(SITE)).await;
    let cookie = service.login().await;
    let empty = service.listed(&cookie, "").await;
    assert_eq!(empty["open"], 0);
    assert_eq!(empty["total"], 0);
    assert_eq!(empty["messages"], serde_json::json!([]));

    let oldest = service.insert("oldest", "30 days", None).await;
    let middle = service.insert("middle", "20 days", Some("5 days")).await;
    let newest = service.insert("newest", "1 day", None).await;

    let all = service.listed(&cookie, "").await;
    assert_eq!(all["open"], 2);
    assert_eq!(all["total"], 3);
    let ids: Vec<&str> = all["messages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        [newest, middle, oldest].map(|id| id.to_string()),
        "newest first"
    );
    let first = &all["messages"][0];
    assert_eq!(first["message"], "newest");
    assert_eq!(first["status"], "new");
    assert_eq!(first["language"], "en");
    assert_eq!(first["name"], serde_json::Value::Null);
    assert_eq!(first["done_at"], serde_json::Value::Null);
    assert!(first["created_at"].as_str().unwrap().ends_with('Z'));

    let new = service.listed(&cookie, "?status=new").await;
    assert_eq!(
        (new["open"].as_i64(), new["total"].as_i64()),
        (Some(2), Some(2))
    );
    assert_eq!(new["messages"].as_array().unwrap().len(), 2);
    assert!(
        new["messages"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["status"] == "new")
    );
    let done = service.listed(&cookie, "?status=done").await;
    assert_eq!(
        (done["open"].as_i64(), done["total"].as_i64()),
        (Some(2), Some(1))
    );
    assert_eq!(done["messages"][0]["message"], "middle");
    assert!(done["messages"][0]["done_at"].as_str().is_some());
    assert_eq!(service.listed(&cookie, "?status=all").await["total"], 3);

    let answer = service
        .call(
            "GET",
            "/ui/api/contact?status=bogus",
            &[("cookie", cookie)],
            None,
        )
        .await;
    assert_eq!(answer.status, 400, "{}", answer.text());
    service.close().await;
}

/// One message is read with its audit, empty until someone changes it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn one_message_is_read_with_an_empty_audit() {
    let service = Service::start("one", Some(SITE)).await;
    let cookie = service.login().await;
    assert_eq!(
        service
            .send("203.0.113.1", &serde_json::json!({}))
            .await
            .status,
        201
    );
    let id = service.listed(&cookie, "").await["messages"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let answer = service
        .call(
            "GET",
            &format!("/ui/api/contact/{id}"),
            &[("cookie", cookie)],
            None,
        )
        .await;
    assert_eq!(answer.status, 200, "{}", answer.text());
    let one = answer.json();
    assert_eq!(one["id"], id.as_str());
    assert_eq!(one["name"], "Max Mustermann");
    assert_eq!(one["email"], "max@example.org");
    assert_eq!(one["message"], "Hallo, ein Spiel hat gehangen.");
    assert_eq!(one["language"], "de");
    assert_eq!(one["status"], "new");
    assert_eq!(one["audit"], serde_json::json!([]));
    service.close().await;
}

/// Marking a message done sets its time and writes one audit row by the
/// admin's name, which never holds the text or the address; marking it new
/// again clears the time.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn marking_a_message_done_is_audited_by_name_without_its_text_and_new_again_clears_the_time()
{
    let service = Service::start("status", Some(SITE)).await;
    let cookie = service.login().await;
    assert_eq!(
        service
            .send("203.0.113.1", &serde_json::json!({}))
            .await
            .status,
        201
    );
    let id = service.listed(&cookie, "").await["messages"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let path = format!("/ui/api/contact/{id}");

    let answer = service
        .call(
            "PATCH",
            &path,
            &service.signed_in(&cookie),
            Some(r#"{"status":"done"}"#.to_owned()),
        )
        .await;
    assert_eq!(answer.status, 200, "{}", answer.text());
    let one = answer.json();
    assert_eq!(one["status"], "done");
    assert!(
        one["done_at"].as_str().is_some_and(|t| t.ends_with('Z')),
        "{one}"
    );
    let audit = one["audit"].as_array().unwrap();
    assert_eq!(audit.len(), 1, "{one}");
    assert_eq!(audit[0]["actor"], NAME);
    assert_eq!(audit[0]["action"], "status");
    assert_eq!(audit[0]["detail"], "done");
    let shown = serde_json::to_string(audit).unwrap();
    assert!(!shown.contains("Hallo"), "the text in the audit: {shown}");
    assert!(
        !shown.contains("max@example.org"),
        "the address in the audit"
    );
    let stored = service.audit_of(&id).await;
    assert_eq!(stored, [format!("{NAME}/status/done")]);
    assert_eq!(service.listed(&cookie, "").await["open"], 0);

    let back = service
        .call(
            "PATCH",
            &path,
            &service.signed_in(&cookie),
            Some(r#"{"status":"new"}"#.to_owned()),
        )
        .await;
    assert_eq!(back.status, 200, "{}", back.text());
    assert_eq!(back.json()["status"], "new");
    assert_eq!(back.json()["done_at"], serde_json::Value::Null);
    assert_eq!(back.json()["audit"].as_array().unwrap().len(), 2);
    assert_eq!(
        service.audit_of(&id).await,
        [format!("{NAME}/status/done"), format!("{NAME}/status/new")]
    );
    assert_eq!(service.listed(&cookie, "").await["open"], 1);
    service.close().await;
}

/// A change wants the UI's CSRF proof and a same-origin page, a status it
/// knows, and a body of exactly its one field.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_change_without_the_proof_or_with_a_bad_status_is_refused_and_changes_nothing() {
    let service = Service::start("badchange", Some(SITE)).await;
    let cookie = service.login().await;
    let id = service.insert("keep me", "1 day", None).await;
    let path = format!("/ui/api/contact/{id}");
    let done = Some(r#"{"status":"done"}"#.to_owned());
    let with_cookie = ("cookie", cookie.clone());
    let origin = ("origin", service.base.clone());
    let csrf = (ui::CSRF_HEADER, "1".to_owned());
    for (why, headers) in [
        ("no csrf header", vec![with_cookie.clone(), origin.clone()]),
        ("no origin", vec![with_cookie.clone(), csrf.clone()]),
        (
            "another origin",
            vec![
                with_cookie.clone(),
                csrf.clone(),
                ("origin", EVIL.to_owned()),
            ],
        ),
    ] {
        for (method, body) in [("PATCH", done.clone()), ("DELETE", None)] {
            let answer = service.call(method, &path, &headers, body).await;
            assert_eq!(answer.status, 403, "{method} {why}");
        }
    }
    let signed_in = service.signed_in(&cookie);
    for (why, body) in [
        ("an unknown status", r#"{"status":"bogus"}"#),
        ("a status in capitals", r#"{"status":"DONE"}"#),
        ("no status", "{}"),
        ("an extra field", r#"{"status":"done","message":"x"}"#),
        ("not json", "done"),
    ] {
        let answer = service
            .call("PATCH", &path, &signed_in, Some(body.to_owned()))
            .await;
        assert_eq!(answer.status, 400, "{why}: {}", answer.text());
    }
    assert_eq!(
        service.listed(&cookie, "").await["messages"][0]["status"],
        "new"
    );
    assert!(service.audit_of(&id.to_string()).await.is_empty());
    service.close().await;
}

/// Deleting removes the message and its text for good; the audit keeps
/// that it was done, by whom, and nothing else.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deleting_a_message_removes_it_and_leaves_an_audit_row_without_its_text() {
    let service = Service::start("delete", Some(SITE)).await;
    let cookie = service.login().await;
    assert_eq!(
        service
            .send("203.0.113.1", &serde_json::json!({}))
            .await
            .status,
        201
    );
    let id = service.listed(&cookie, "").await["messages"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let path = format!("/ui/api/contact/{id}");
    let answer = service
        .call("DELETE", &path, &service.signed_in(&cookie), None)
        .await;
    assert_eq!(answer.status, 204, "{}", answer.text());
    assert_eq!(service.stored().await, 0);
    let answer = service
        .call("GET", &path, &[("cookie", cookie.clone())], None)
        .await;
    assert_eq!(answer.status, 404);
    assert_eq!(service.audit_of(&id).await, [format!("{NAME}/delete/")]);
    let all = service
        .count("SELECT count(*) AS n FROM feedback_audit WHERE detail LIKE '%Hallo%' OR actor LIKE '%@%'")
        .await;
    assert_eq!(all, 0, "no text or address in any audit row");
    // The second delete finds nothing, and leaves no second row.
    let again = service
        .call("DELETE", &path, &service.signed_in(&cookie), None)
        .await;
    assert_eq!(again.status, 404);
    assert_eq!(service.audit_of(&id).await.len(), 1);
    service.close().await;
}

/// An id that is no message, or no id, is not found.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unknown_or_garbage_id_is_not_found() {
    let service = Service::start("unknown", Some(SITE)).await;
    let cookie = service.login().await;
    let signed_in = service.signed_in(&cookie);
    for id in [
        Uuid::now_v7().to_string(),
        "not-a-uuid".to_owned(),
        "0".to_owned(),
        "..%2Fme".to_owned(),
    ] {
        let path = format!("/ui/api/contact/{id}");
        for (method, body) in [
            ("GET", None),
            ("PATCH", Some(r#"{"status":"done"}"#.to_owned())),
            ("DELETE", None),
        ] {
            let answer = service.call(method, &path, &signed_in, body).await;
            assert_eq!(answer.status, 404, "{method} {id}: {}", answer.text());
        }
    }
    assert_eq!(
        service
            .count("SELECT count(*) AS n FROM feedback_audit")
            .await,
        0,
        "nothing happened, nothing audited"
    );
    service.close().await;
}

// ------------------------------------------------------------------ retention

/// A message is deleted a year after it came, and three months after it
/// was marked done; reading the list is when the sweep runs.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reading_the_list_deletes_messages_past_their_time_and_keeps_the_rest() {
    let service = Service::start("sweep_list", Some(SITE)).await;
    let cookie = service.login().await;
    let year = service.insert("a year old", "400 days", None).await;
    let done_long_ago = service
        .insert("done long ago", "150 days", Some("100 days"))
        .await;
    let done_lately = service
        .insert("done lately", "20 days", Some("10 days"))
        .await;
    let recent = service.insert("recent", "1 day", None).await;
    let old_but_open = service.insert("open, a month old", "30 days", None).await;
    assert_eq!(service.stored().await, 5);

    let list = service.listed(&cookie, "").await;
    assert_eq!(list["total"], 3, "{list}");
    assert_eq!(list["open"], 2);
    let mut left: Vec<String> = list["messages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["id"].as_str().unwrap().to_owned())
        .collect();
    left.sort();
    let mut kept = [done_lately, recent, old_but_open].map(|id| id.to_string());
    kept.sort();
    assert_eq!(left, kept);
    assert_eq!(service.stored().await, 3);
    for gone in [year, done_long_ago] {
        let answer = service
            .call(
                "GET",
                &format!("/ui/api/contact/{gone}"),
                &[("cookie", cookie.clone())],
                None,
            )
            .await;
        assert_eq!(answer.status, 404, "{gone}");
    }
    service.close().await;
}

/// Taking a new message sweeps too, so nothing old outlives a quiet admin.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn taking_a_message_deletes_messages_past_their_time() {
    let service = Service::start("sweep_post", Some(SITE)).await;
    service.insert("a year old", "400 days", None).await;
    service
        .insert("done long ago", "150 days", Some("100 days"))
        .await;
    service
        .insert("done lately", "20 days", Some("10 days"))
        .await;
    assert_eq!(service.stored().await, 3);
    let answer = service.send("203.0.113.1", &serde_json::json!({})).await;
    assert_eq!(answer.status, 201, "{}", answer.text());
    assert_eq!(service.stored().await, 2, "the two old ones went");
    assert_eq!(
        service
            .count("SELECT count(*) AS n FROM feedback_contact WHERE message = 'done lately'")
            .await,
        1
    );
    service.close().await;
}

// ------------------------------------------------------------------ the console

/// The console's audit list is about the console; a message's audit rows
/// are shown with the message and nowhere else.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_console_audit_does_not_show_changes_to_contact_messages() {
    let service = Service::start("console", Some(SITE)).await;
    let cookie = service.login().await;
    let id = service.insert("hi", "1 day", None).await;
    let answer = service
        .call(
            "PATCH",
            &format!("/ui/api/contact/{id}"),
            &service.signed_in(&cookie),
            Some(r#"{"status":"done"}"#.to_owned()),
        )
        .await;
    assert_eq!(answer.status, 200, "{}", answer.text());
    let answer = service
        .call(
            "DELETE",
            &format!("/ui/api/contact/{id}"),
            &service.signed_in(&cookie),
            None,
        )
        .await;
    assert_eq!(answer.status, 204);
    assert_eq!(service.audit_of(&id.to_string()).await.len(), 2);

    let console = || async {
        service
            .call(
                "GET",
                "/ui/api/admin/audit",
                &[("cookie", cookie.clone())],
                None,
            )
            .await
    };
    let answer = console().await;
    assert_eq!(answer.status, 200, "{}", answer.text());
    assert_eq!(
        answer.json(),
        serde_json::json!([]),
        "no console change yet"
    );

    // A console change (no report, no message) is still listed.
    service
        .db
        .execute_unprepared(&format!(
            "INSERT INTO feedback_audit (id, actor, action, detail) \
             VALUES ('{}', '{NAME}', 'gateway.invite.create', 'main/x')",
            Uuid::now_v7()
        ))
        .await
        .unwrap();
    let answer = console().await;
    let rows = answer.json();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0]["action"], "gateway.invite.create");
    service.close().await;
}
