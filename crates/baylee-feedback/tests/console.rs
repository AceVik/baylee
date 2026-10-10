//! The admin console's road through the service (`docs/feedback.md` §"The
//! admin console"): `/ui/api/admin/…` against a stand-in gateway that
//! records what reached it, over real HTTP, with a real PostgreSQL for the
//! sessions and the audit.
//!
//! What they hold: nothing reaches the gateway without a session; a change
//! also wants the UI's CSRF proof; what reaches it carries the service's
//! token and the admin's name and nothing the page chose beyond the four
//! fields; the gateway refusing the token is not a sign-out; and the audit
//! names ids, never a key.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU16, Ordering};
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
const KEY: &str = "BAYLEE-ABCD-EFGH-JKMN-PQRS";
const KEY_ID: &str = "0199aaaa-0000-7000-8000-0000000000aa";

/// What reached the stand-in gateway.
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
    forced: Arc<AtomicU16>,
    server: tokio::task::JoinHandle<()>,
}

async fn stub() -> Stub {
    let seen: Arc<Mutex<Vec<Seen>>> = Arc::default();
    let forced = Arc::new(AtomicU16::new(0));
    let (log, force) = (seen.clone(), forced.clone());
    let app = Router::new().fallback(
        move |method: Method, uri: Uri, headers: HeaderMap, body: Bytes| {
            let (log, force) = (log.clone(), force.clone());
            async move {
                log.lock().unwrap().push(Seen {
                    method: method.to_string(),
                    path: uri
                        .path_and_query()
                        .map_or_else(|| uri.path().to_owned(), ToString::to_string),
                    headers: headers
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_owned()))
                        .collect(),
                    body: String::from_utf8_lossy(&body).into_owned(),
                });
                let forced = force.load(Ordering::SeqCst);
                if forced != 0 {
                    return (
                        StatusCode::from_u16(forced).unwrap(),
                        axum::Json(serde_json::json!({ "error": "forced" })),
                    )
                        .into_response();
                }
                match (method.as_str(), uri.path()) {
                    ("GET", "/admin/stats") => {
                        axum::Json(serde_json::json!({ "accounts": { "registered": 3 } }))
                            .into_response()
                    }
                    ("GET", "/admin/invites") => axum::Json(serde_json::json!([])).into_response(),
                    ("GET", path)
                        if path == "/admin/live"
                            || path == "/admin/metrics"
                            || path.starts_with("/admin/sets")
                            || path.starts_with("/admin/accounts") =>
                    {
                        axum::Json(serde_json::json!({})).into_response()
                    }
                    ("POST", "/admin/invites") => (
                        StatusCode::CREATED,
                        axum::Json(serde_json::json!({
                            "keys": [{ "id": KEY_ID, "key": KEY }],
                            "uses": 1,
                            "expires_at": null,
                            "note": "for Max",
                        })),
                    )
                        .into_response(),
                    ("DELETE", path) if path.starts_with("/admin/invites/") => {
                        StatusCode::NO_CONTENT.into_response()
                    }
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
}

struct Answer {
    status: u16,
    body: Vec<u8>,
    /// `name=value` of a cookie the answer set.
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
    /// The service in a schema of its own, talking to `gateway` if given.
    async fn start(what: &str, gateway: Option<&str>) -> Self {
        let url = std::env::var("DATABASE_URL")
            .ok()
            .filter(|u| !u.is_empty())
            .expect("DATABASE_URL is not set, and these tests are about PostgreSQL");
        let schema: String = format!("fbcon_{what}_{}", Uuid::now_v7().simple())
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
        let mut config = baylee_feedback::Config::new(
            "eu=gateway-token-eu-0001",
            Some("read-token-000000001"),
            Some("admin-token-00000001"),
        )
        .expect("config");
        if let Some(gateway) = gateway {
            config = config
                .with_gateway_admin(gateway, CONSOLE_TOKEN)
                .expect("a console");
        }
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

    /// The headers this UI's own pages send with a change.
    fn proof(&self) -> Vec<(&'static str, String)> {
        vec![
            ("origin", self.base.clone()),
            (ui::CSRF_HEADER, "1".to_owned()),
        ]
    }

    async fn login(&self) -> String {
        let answer = self
            .call(
                "POST",
                "/ui/api/login",
                &self.proof(),
                Some(&serde_json::json!({ "name": NAME, "password": PASSWORD }).to_string()),
            )
            .await;
        assert_eq!(answer.status, 200, "{}", answer.text());
        answer.cookie.expect("a session cookie")
    }
}

fn with_cookie(cookie: &str) -> Vec<(&'static str, String)> {
    vec![("cookie", cookie.to_owned())]
}

#[tokio::test(flavor = "multi_thread")]
async fn nothing_reaches_the_gateway_without_a_session() {
    let gw = stub().await;
    let service = Service::start("nosession", Some(&gw.url)).await;
    for (method, path, body) in [
        ("GET", "/ui/api/admin/stats", None),
        ("GET", "/ui/api/admin/invites", None),
        ("GET", "/ui/api/admin/audit", None),
        ("GET", "/ui/api/admin/live", None),
        ("GET", "/ui/api/admin/metrics", None),
        ("GET", "/ui/api/admin/sets", None),
        ("GET", "/ui/api/admin/sets/lea", None),
        ("GET", "/ui/api/admin/accounts", None),
        ("GET", &format!("/ui/api/admin/accounts/{KEY_ID}"), None),
        ("POST", "/ui/api/admin/invites", Some("{}")),
        ("DELETE", &format!("/ui/api/admin/invites/{KEY_ID}"), None),
    ] {
        let mut headers = service.proof();
        let answer = service.call(method, path, &headers, body).await;
        assert_eq!(answer.status, 401, "{method} {path}: {}", answer.text());
        // The service's own tokens open none of these either.
        for token in [
            "read-token-000000001",
            "admin-token-00000001",
            CONSOLE_TOKEN,
        ] {
            headers.push(("authorization", format!("Bearer {token}")));
            let answer = service.call(method, path, &headers, body).await;
            assert_eq!(answer.status, 401, "{method} {path} with a token");
            headers.pop();
        }
    }
    assert!(gw.seen().is_empty(), "{:?}", gw.seen());
    service.close().await;
    gw.server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_signed_in_admin_reaches_the_gateway_with_the_services_token_and_name() {
    let gw = stub().await;
    let service = Service::start("reads", Some(&gw.url)).await;
    let cookie = service.login().await;
    let me = service
        .call("GET", "/ui/api/me", &with_cookie(&cookie), None)
        .await;
    assert_eq!(me.json()["gateway_admin"], true, "{}", me.text());

    let answer = service
        .call("GET", "/ui/api/admin/stats", &with_cookie(&cookie), None)
        .await;
    assert_eq!(answer.status, 200, "{}", answer.text());
    assert_eq!(answer.json()["accounts"]["registered"], 3);
    let answer = service
        .call("GET", "/ui/api/admin/invites", &with_cookie(&cookie), None)
        .await;
    assert_eq!((answer.status, answer.text().as_str()), (200, "[]"));

    let seen = gw.seen();
    assert_eq!(seen.len(), 2, "{seen:?}");
    for request in &seen {
        assert_eq!(
            request.header("authorization"),
            Some(format!("Bearer {CONSOLE_TOKEN}").as_str())
        );
        assert_eq!(request.header("x-baylee-admin"), Some(NAME));
        for kept_back in ["cookie", "origin", "sec-fetch-site", ui::CSRF_HEADER] {
            assert_eq!(request.header(kept_back), None, "{kept_back} passed on");
        }
    }
    assert_eq!(seen[0].path, "/admin/stats");
    assert_eq!(seen[1].path, "/admin/invites");
    service.close().await;
    gw.server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_people_pages_pass_on_only_the_fields_they_read() {
    let gw = stub().await;
    let service = Service::start("people", Some(&gw.url)).await;
    let cookie = service.login().await;
    for path in [
        "/ui/api/admin/live".to_owned(),
        "/ui/api/admin/accounts?q=Al%20ice&kind=guest&online=true&offset=50&limit=25".to_owned(),
        format!("/ui/api/admin/accounts/{KEY_ID}"),
        "/ui/api/admin/metrics".to_owned(),
        "/ui/api/admin/sets".to_owned(),
        "/ui/api/admin/sets/LEA".to_owned(),
    ] {
        let answer = service
            .call("GET", &path, &with_cookie(&cookie), None)
            .await;
        assert_eq!(answer.status, 200, "{path}: {}", answer.text());
    }
    for refused in [
        "/ui/api/admin/accounts?admin=1",
        "/ui/api/admin/accounts?limit=-1",
        "/ui/api/admin/accounts/nobody",
        "/ui/api/admin/sets/a-b",
        "/ui/api/admin/sets/%2e%2e",
        "/ui/api/admin/sets/toolongforaset",
    ] {
        let answer = service
            .call("GET", refused, &with_cookie(&cookie), None)
            .await;
        assert_eq!(answer.status, 400, "{refused}: {}", answer.text());
    }
    let seen: Vec<String> = gw.seen().into_iter().map(|s| s.path).collect();
    assert_eq!(
        seen,
        [
            "/admin/live".to_owned(),
            "/admin/accounts?q=Al%20ice&kind=guest&online=true&offset=50&limit=25".to_owned(),
            format!("/admin/accounts/{KEY_ID}"),
            "/admin/metrics".to_owned(),
            "/admin/sets".to_owned(),
            // Lower-cased here, so the gateway sees one spelling.
            "/admin/sets/lea".to_owned(),
        ]
    );
    service.close().await;
    gw.server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_change_wants_the_proof_passes_only_its_fields_and_is_audited_without_a_key() {
    let gw = stub().await;
    let service = Service::start("writes", Some(&gw.url)).await;
    let cookie = service.login().await;
    let order = r#"{"count":2,"note":"for Max","expires":"30d","uses":1}"#;
    let delete = format!("/ui/api/admin/invites/{KEY_ID}");

    // No header, a foreign origin, or a cross-site fetch: refused here.
    let bare = with_cookie(&cookie);
    let mut foreign = with_cookie(&cookie);
    foreign.push(("origin", "https://evil.example".into()));
    foreign.push((ui::CSRF_HEADER, "1".into()));
    let mut cross = service.proof();
    cross.push(("cookie", cookie.clone()));
    cross.push(("sec-fetch-site", "cross-site".into()));
    for headers in [&bare, &foreign, &cross] {
        let answer = service
            .call("POST", "/ui/api/admin/invites", headers, Some(order))
            .await;
        assert_eq!(answer.status, 403, "{}", answer.text());
        let answer = service.call("DELETE", &delete, headers, None).await;
        assert_eq!(answer.status, 403, "{}", answer.text());
    }
    assert!(gw.seen().is_empty(), "{:?}", gw.seen());

    let mut proved = service.proof();
    proved.push(("cookie", cookie.clone()));
    // Only the four fields, and only an id.
    for body in [
        r#"{"count":1,"admin":"someone"}"#,
        "not json",
        r#"{"uses":-1}"#,
    ] {
        let answer = service
            .call("POST", "/ui/api/admin/invites", &proved, Some(body))
            .await;
        assert_eq!(answer.status, 400, "{body}: {}", answer.text());
    }
    let answer = service
        .call("DELETE", "/ui/api/admin/invites/..%2Fstats", &proved, None)
        .await;
    assert_eq!(answer.status, 400);
    assert!(gw.seen().is_empty(), "{:?}", gw.seen());

    let answer = service
        .call("POST", "/ui/api/admin/invites", &proved, Some(order))
        .await;
    assert_eq!(answer.status, 201, "{}", answer.text());
    assert_eq!(answer.json()["keys"][0]["key"], KEY, "the key, shown once");
    let answer = service.call("DELETE", &delete, &proved, None).await;
    assert_eq!(answer.status, 204, "{}", answer.text());

    let seen = gw.seen();
    assert_eq!(seen.len(), 2, "{seen:?}");
    assert_eq!(
        (seen[0].method.as_str(), seen[0].path.as_str()),
        ("POST", "/admin/invites")
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&seen[0].body).unwrap(),
        serde_json::json!({ "uses": 1, "expires": "30d", "note": "for Max", "count": 2 })
    );
    assert_eq!(seen[0].header("x-baylee-admin"), Some(NAME));
    assert_eq!(
        (seen[1].method.as_str(), seen[1].path.as_str()),
        ("DELETE", format!("/admin/invites/{KEY_ID}").as_str())
    );

    let audit = service
        .call("GET", "/ui/api/admin/audit", &with_cookie(&cookie), None)
        .await;
    assert_eq!(audit.status, 200);
    let rows = audit.json();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_eq!(rows[0]["action"], "gateway.invite.revoke", "newest first");
    assert_eq!(rows[1]["action"], "gateway.invite.create");
    for row in rows {
        assert_eq!(row["actor"], NAME);
        assert!(row["detail"].as_str().unwrap().contains(KEY_ID), "{row}");
    }
    assert!(!audit.text().contains(KEY), "a key in the audit");
    assert!(!audit.text().contains("for Max"), "a note in the audit");
    service.close().await;
    gw.server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_gateway_refusing_the_token_is_not_a_sign_out() {
    let gw = stub().await;
    let service = Service::start("refused", Some(&gw.url)).await;
    let cookie = service.login().await;
    for (forced, ours) in [(401, 502), (403, 502), (429, 503), (500, 502)] {
        gw.forced.store(forced, Ordering::SeqCst);
        let answer = service
            .call("GET", "/ui/api/admin/stats", &with_cookie(&cookie), None)
            .await;
        assert_eq!(answer.status, ours, "the gateway said {forced}");
        assert!(!answer.text().contains(CONSOLE_TOKEN));
    }
    let me = service
        .call("GET", "/ui/api/me", &with_cookie(&cookie), None)
        .await;
    assert_eq!(me.status, 200, "still signed in");
    // A gateway that is not there at all.
    gw.server.abort();
    let _ = gw.server.await;
    let answer = service
        .call("GET", "/ui/api/admin/stats", &with_cookie(&cookie), None)
        .await;
    assert_eq!(answer.status, 502, "{}", answer.text());
    service.close().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn without_a_gateway_the_admin_pages_are_off() {
    let service = Service::start("off", None).await;
    let cookie = service.login().await;
    let me = service
        .call("GET", "/ui/api/me", &with_cookie(&cookie), None)
        .await;
    assert_eq!(me.json()["gateway_admin"], false, "{}", me.text());
    let answer = service
        .call("GET", "/ui/api/admin/stats", &with_cookie(&cookie), None)
        .await;
    assert_eq!(answer.status, 404);
    service.close().await;
}

#[test]
fn the_console_token_is_no_other_token_of_the_service() {
    let key = "a-direct-key-that-is-long-enough-000000000";
    let base = || {
        baylee_feedback::Config::new(
            "eu=gateway-token-eu-0001",
            Some("read-token-000000001"),
            Some("admin-token-00000001"),
        )
        .unwrap()
    };
    let url = "http://127.0.0.1:28767";
    assert!(base().with_gateway_admin(url, CONSOLE_TOKEN).is_ok());
    assert!(
        base()
            .with_direct_key(key)
            .unwrap()
            .with_gateway_admin(url, key)
            .is_err()
    );
    assert!(
        base()
            .with_gateway_admin(url, key)
            .unwrap()
            .with_direct_key(key)
            .is_err()
    );
    assert!(base().with_gateway_admin(url, "short-token").is_err());
    assert!(
        base()
            .with_gateway_admin("127.0.0.1:28767", CONSOLE_TOKEN)
            .is_err()
    );
}
