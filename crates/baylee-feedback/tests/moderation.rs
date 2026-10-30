//! Acting on an account or a game through the admin pages (`docs/feedback.md`
//! §"The admin console", beta.8): `/ui/api/admin/accounts/{id}/…` and
//! `/ui/api/admin/games/{id}/close` against a stand-in gateway that records
//! what reached it, over real HTTP, with a real PostgreSQL for the sessions
//! and the audit.
//!
//! What they hold: nothing is passed on without a session, and a change
//! wants the UI's CSRF proof; ids are ids, and a body carries only the
//! fields the page has; what reaches the gateway carries the admin's name;
//! the gateway's refusals are the page's; and an act that worked is one
//! audit row naming the account or the game, never a reason. There is no
//! password reset: accounts sign in at Baylee Central.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Bytes;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::IntoResponse as _;
use baylee_feedback::ui::{self, Ui};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use uuid::Uuid;

const NAME: &str = "viktor";
const PASSWORD: &str = "a long enough password";
const CONSOLE_TOKEN: &str = "the-gateways-admin-token-0123456789abcdef";
const ACCOUNT: &str = "0199aaaa-0000-7000-8000-0000000000aa";
const GAME: &str = "0199bbbb-0000-7000-8000-0000000000bb";

#[derive(Clone, Debug)]
struct Seen {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: String,
}

impl Seen {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// A gateway's console as far as the service can tell: it records every
/// request and answers as `/admin/…` does, or with `forced` when set.
struct Stub {
    url: String,
    seen: Arc<Mutex<Vec<Seen>>>,
    forced: Arc<Mutex<u16>>,
    server: tokio::task::JoinHandle<()>,
}

async fn stub() -> Stub {
    let seen: Arc<Mutex<Vec<Seen>>> = Arc::default();
    let forced = Arc::new(Mutex::new(0u16));
    let (log, force) = (seen.clone(), forced.clone());
    let app = Router::new().fallback(
        move |method: Method, uri: Uri, headers: HeaderMap, body: Bytes| {
            let (log, force) = (log.clone(), force.clone());
            async move {
                let text = String::from_utf8_lossy(&body).into_owned();
                log.lock().unwrap().push(Seen {
                    method: method.to_string(),
                    path: uri.path().to_owned(),
                    headers: headers
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_owned()))
                        .collect(),
                    body: text.clone(),
                });
                let forced = *force.lock().unwrap();
                if forced != 0 {
                    return (
                        StatusCode::from_u16(forced).unwrap(),
                        axum::Json(serde_json::json!({ "error": "forced" })),
                    )
                        .into_response();
                }
                let path = uri.path();
                match method.as_str() {
                    "PUT" if path.ends_with("/ban") => {
                        let order: serde_json::Value =
                            serde_json::from_str(&text).unwrap_or_default();
                        axum::Json(serde_json::json!({
                            "ban": { "reason": order["reason"], "until": order["until"] }
                        }))
                        .into_response()
                    }
                    "DELETE" => StatusCode::NO_CONTENT.into_response(),
                    "POST" if path.ends_with("/close") => StatusCode::ACCEPTED.into_response(),
                    _ => StatusCode::NOT_FOUND.into_response(),
                }
            }
        },
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Stub {
        url: format!("http://{addr}"),
        seen,
        forced,
        server,
    }
}

impl Stub {
    fn seen(&self) -> Vec<Seen> {
        self.seen.lock().unwrap().clone()
    }
    fn force(&self, status: u16) {
        *self.forced.lock().unwrap() = status;
    }
}

struct Answer {
    status: u16,
    body: Vec<u8>,
    cookie: Option<String>,
}

impl Answer {
    fn json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body).unwrap_or(serde_json::Value::Null)
    }
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
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
    async fn start(what: &str, gateway: &str) -> Self {
        let url = std::env::var("DATABASE_URL")
            .ok()
            .filter(|u| !u.is_empty())
            .expect("DATABASE_URL is not set, and these tests are about PostgreSQL");
        let schema: String = format!("fbmod_{what}_{}", Uuid::now_v7().simple())
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
        let config = baylee_feedback::Config::new(
            "eu=gateway-token-eu-0001",
            Some("read-token-000000001"),
            Some("admin-token-00000001"),
        )
        .expect("config")
        .with_gateway_admin(gateway, CONSOLE_TOKEN)
        .expect("a console");
        let app = baylee_feedback::app_with(
            Arc::new(baylee_feedback::AppState {
                db: db.clone(),
                config,
            }),
            Ui::default(),
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

    async fn call(
        &self,
        method: &str,
        path: &str,
        headers: &[(&str, String)],
        body: Option<&str>,
    ) -> Answer {
        let url = format!("{}{path}", self.base);
        let method = method.to_owned();
        let headers: Vec<(String, String)> = headers
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect();
        let body = body.map(str::to_owned);
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
            let cookie = response
                .headers()
                .get("set-cookie")
                .and_then(|v| v.to_str().ok())
                .and_then(|set| set.split(';').next())
                .map(|pair| pair.trim().to_owned());
            Answer {
                status: response.status().as_u16(),
                body: response.body_mut().read_to_vec().unwrap_or_default(),
                cookie,
            }
        })
        .await
        .unwrap()
    }

    fn proof(&self) -> Vec<(&'static str, String)> {
        vec![
            ("origin", self.base.clone()),
            (ui::CSRF_HEADER, "1".to_owned()),
        ]
    }

    /// A signed-in admin's headers for a change.
    async fn proved(&self) -> Vec<(&'static str, String)> {
        let answer = self
            .call(
                "POST",
                "/ui/api/login",
                &self.proof(),
                Some(&serde_json::json!({ "name": NAME, "password": PASSWORD }).to_string()),
            )
            .await;
        assert_eq!(answer.status, 200, "{}", answer.text());
        let mut headers = self.proof();
        headers.push(("cookie", answer.cookie.expect("a session cookie")));
        headers
    }

    /// The audit rows, newest first.
    async fn audit(&self, headers: &[(&'static str, String)]) -> (Vec<serde_json::Value>, String) {
        let cookie = headers.iter().find(|(k, _)| *k == "cookie").unwrap();
        let answer = self
            .call(
                "GET",
                "/ui/api/admin/audit",
                &[("cookie", cookie.1.clone())],
                None,
            )
            .await;
        assert_eq!(answer.status, 200, "{}", answer.text());
        (answer.json().as_array().unwrap().clone(), answer.text())
    }
}

/// Every act of the page, with a body that is good.
fn acts() -> Vec<(&'static str, String, Option<&'static str>)> {
    vec![
        (
            "PUT",
            format!("/ui/api/admin/accounts/{ACCOUNT}/ban"),
            Some(r#"{"reason":"spam"}"#),
        ),
        (
            "DELETE",
            format!("/ui/api/admin/accounts/{ACCOUNT}/ban"),
            None,
        ),
        ("DELETE", format!("/ui/api/admin/accounts/{ACCOUNT}"), None),
        (
            "POST",
            format!("/ui/api/admin/games/{GAME}/close"),
            Some(r#"{"reason":"stuck"}"#),
        ),
    ]
}

#[tokio::test(flavor = "multi_thread")]
async fn an_act_wants_a_session_and_the_proof_and_nothing_passes_before() {
    let gw = stub().await;
    let service = Service::start("auth", &gw.url).await;
    let signed_in = service.proved().await;
    let cookie = signed_in
        .iter()
        .find(|(k, _)| *k == "cookie")
        .unwrap()
        .1
        .clone();

    for (method, path, body) in acts() {
        // No session, even with the proof.
        let answer = service.call(method, &path, &service.proof(), body).await;
        assert_eq!(answer.status, 401, "{method} {path}: {}", answer.text());
        // A session without the proof, or with a foreign origin, or fetched
        // across sites.
        let bare = vec![("cookie", cookie.clone())];
        let mut foreign = bare.clone();
        foreign.push(("origin", "https://evil.example".into()));
        foreign.push((ui::CSRF_HEADER, "1".into()));
        let mut cross = signed_in.clone();
        cross.push(("sec-fetch-site", "cross-site".into()));
        for headers in [&bare, &foreign, &cross] {
            let answer = service.call(method, &path, headers, body).await;
            assert_eq!(answer.status, 403, "{method} {path}: {}", answer.text());
        }
        // The service's own tokens are no admin's session.
        for token in [
            "read-token-000000001",
            "admin-token-00000001",
            CONSOLE_TOKEN,
        ] {
            let mut headers = service.proof();
            headers.push(("authorization", format!("Bearer {token}")));
            let answer = service.call(method, &path, &headers, body).await;
            assert_eq!(answer.status, 401, "{method} {path} with a token");
        }
    }
    assert!(gw.seen().is_empty(), "{:?}", gw.seen());
    let (rows, _) = service.audit(&signed_in).await;
    assert!(rows.is_empty(), "{rows:?}");
    service.close().await;
    gw.server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn an_id_is_an_id_and_a_body_has_only_the_pages_fields() {
    let gw = stub().await;
    let service = Service::start("shape", &gw.url).await;
    let proved = service.proved().await;
    for (method, path) in [
        ("PUT", "/ui/api/admin/accounts/nobody/ban"),
        ("DELETE", "/ui/api/admin/accounts/nobody/ban"),
        ("DELETE", "/ui/api/admin/accounts/nobody"),
        ("DELETE", "/ui/api/admin/accounts/..%2Fstats"),
        ("POST", "/ui/api/admin/games/nobody/close"),
    ] {
        let answer = service
            .call(method, path, &proved, Some(r#"{"reason":"x"}"#))
            .await;
        assert_eq!(answer.status, 400, "{method} {path}: {}", answer.text());
    }
    let ban = format!("/ui/api/admin/accounts/{ACCOUNT}/ban");
    let close = format!("/ui/api/admin/games/{GAME}/close");
    for (path, method, body) in [
        (&ban, "PUT", r#"{"reason":"x","admin":"someone"}"#),
        (&ban, "PUT", r#"{"until":"2999-01-01T00:00:00Z"}"#),
        (&ban, "PUT", "not json"),
        (&close, "POST", r#"{"reason":"x","force":true}"#),
        (&close, "POST", "{}"),
        (&close, "POST", "not json"),
    ] {
        let answer = service.call(method, path, &proved, Some(body)).await;
        assert_eq!(
            answer.status,
            400,
            "{method} {path} {body}: {}",
            answer.text()
        );
    }
    assert!(gw.seen().is_empty(), "{:?}", gw.seen());
    let (rows, _) = service.audit(&proved).await;
    assert!(rows.is_empty(), "a refusal is no act: {rows:?}");
    service.close().await;
    gw.server.abort();
}

#[tokio::test(flavor = "multi_thread")]
#[allow(clippy::too_many_lines)] // one scenario, read top to bottom
async fn each_act_reaches_the_gateway_as_the_admin_and_is_one_audit_row_without_its_words() {
    let gw = stub().await;
    let service = Service::start("acts", &gw.url).await;
    let proved = service.proved().await;
    let reason = "a private grievance about Max";
    let closing = "a reason naming Erika";

    let ban = service
        .call(
            "PUT",
            &format!("/ui/api/admin/accounts/{ACCOUNT}/ban"),
            &proved,
            Some(&format!(
                r#"{{"reason":"{reason}","until":"2999-01-01T00:00:00Z"}}"#
            )),
        )
        .await;
    assert_eq!(ban.status, 200, "{}", ban.text());
    assert_eq!(ban.json()["ban"]["reason"], reason);
    let forever = service
        .call(
            "PUT",
            &format!("/ui/api/admin/accounts/{ACCOUNT}/ban"),
            &proved,
            Some(&format!(r#"{{"reason":"{reason}"}}"#)),
        )
        .await;
    assert_eq!(forever.status, 200, "{}", forever.text());

    let lifted = service
        .call(
            "DELETE",
            &format!("/ui/api/admin/accounts/{ACCOUNT}/ban"),
            &proved,
            None,
        )
        .await;
    assert_eq!(lifted.status, 204, "{}", lifted.text());
    let deleted = service
        .call(
            "DELETE",
            &format!("/ui/api/admin/accounts/{ACCOUNT}"),
            &proved,
            None,
        )
        .await;
    assert_eq!(deleted.status, 204, "{}", deleted.text());
    let closed = service
        .call(
            "POST",
            &format!("/ui/api/admin/games/{GAME}/close"),
            &proved,
            Some(&format!(r#"{{"reason":"{closing}"}}"#)),
        )
        .await;
    assert_eq!(closed.status, 202, "{}", closed.text());

    let seen = gw.seen();
    let routes: Vec<String> = seen
        .iter()
        .map(|s| format!("{} {}", s.method, s.path))
        .collect();
    assert_eq!(
        routes,
        [
            format!("PUT /admin/accounts/{ACCOUNT}/ban"),
            format!("PUT /admin/accounts/{ACCOUNT}/ban"),
            format!("DELETE /admin/accounts/{ACCOUNT}/ban"),
            format!("DELETE /admin/accounts/{ACCOUNT}"),
            format!("POST /admin/games/{GAME}/close"),
        ]
    );
    for request in &seen {
        assert_eq!(
            request.header("authorization"),
            Some(format!("Bearer {CONSOLE_TOKEN}").as_str())
        );
        assert_eq!(request.header("x-baylee-admin"), Some(NAME));
        for kept_back in ["cookie", "origin", ui::CSRF_HEADER] {
            assert_eq!(request.header(kept_back), None, "{kept_back} passed on");
        }
    }
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&seen[0].body).unwrap(),
        serde_json::json!({ "reason": reason, "until": "2999-01-01T00:00:00Z" })
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&seen[1].body).unwrap(),
        serde_json::json!({ "reason": reason }),
        "a permanent ban sends no end"
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&seen[4].body).unwrap(),
        serde_json::json!({ "reason": closing })
    );

    let (rows, text) = service.audit(&proved).await;
    let actions: Vec<&str> = rows.iter().map(|r| r["action"].as_str().unwrap()).collect();
    assert_eq!(
        actions,
        [
            "gateway.game.close",
            "gateway.account.delete",
            "gateway.account.unban",
            "gateway.account.ban",
            "gateway.account.ban",
        ],
        "newest first, one row for each act"
    );
    for row in &rows {
        assert_eq!(row["actor"], NAME);
        let detail = row["detail"].as_str().unwrap();
        let id = if row["action"] == "gateway.game.close" {
            GAME
        } else {
            ACCOUNT
        };
        assert!(detail.contains(id), "{row}");
    }
    assert!(
        rows[3]["detail"].as_str().unwrap().contains("until never"),
        "{}",
        rows[3]
    );
    assert!(
        rows[4]["detail"]
            .as_str()
            .unwrap()
            .contains("until 2999-01-01T00:00:00Z"),
        "{}",
        rows[4]
    );
    for secret in [reason, closing, "Max", "Erika", CONSOLE_TOKEN] {
        assert!(!text.contains(secret), "{secret} in the audit: {text}");
    }
    service.close().await;
    gw.server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_gateways_refusals_are_the_pages_and_leave_no_audit_row() {
    let gw = stub().await;
    let service = Service::start("refused", &gw.url).await;
    let proved = service.proved().await;
    for (forced, ours) in [(404, 404), (409, 409), (400, 400), (503, 503)] {
        gw.force(forced);
        for (method, path, body) in acts() {
            let answer = service.call(method, &path, &proved, body).await;
            assert_eq!(answer.status, ours, "{method} {path}: {}", answer.text());
        }
    }
    // A console token the gateway refuses is a broken service, not a
    // player's mistake, and not a sign-out.
    for (forced, ours) in [(401, 502), (403, 502), (429, 503)] {
        gw.force(forced);
        let (method, path, body) = acts().remove(0);
        let answer = service.call(method, &path, &proved, body).await;
        assert_eq!(answer.status, ours, "the gateway said {forced}");
        assert!(!answer.text().contains(CONSOLE_TOKEN));
    }
    let (rows, _) = service.audit(&proved).await;
    assert!(rows.is_empty(), "nothing was done: {rows:?}");
    service.close().await;
    gw.server.abort();
}
