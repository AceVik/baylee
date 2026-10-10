//! The web UI's half of the service (#311) against a real PostgreSQL, over
//! real HTTP: the admin CLI, sign-in, sessions and their clock, the CSRF
//! rule, the UI's JSON routes, and the static files.
//!
//! Each test runs in a schema of its own, as `service.rs` does; without
//! `DATABASE_URL` they fail rather than skip.

use std::io::{Read as _, Write as _};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use baylee_feedback::ui::{self, Ui};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

const GATEWAY: &str = "gateway-token-eu-0001";
const US: &str = "gateway-token-us-0002";
const READ: &str = "read-token-000000001";
const ADMIN_TOKEN: &str = "admin-token-00000001";

const NAME: &str = "viktor";
const PASSWORD: &str = "a long enough password";

/// A clock the test moves.
#[derive(Clone)]
struct Hands(Arc<Mutex<OffsetDateTime>>);

impl Hands {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(
            OffsetDateTime::from_unix_timestamp(1_790_000_000).unwrap(),
        )))
    }
    fn advance(&self, by: Duration) {
        *self.0.lock().unwrap() += by;
    }
    fn clock(&self) -> ui::Clock {
        let hands = self.0.clone();
        Arc::new(move || *hands.lock().unwrap())
    }
}

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
    /// `name=value` of the session cookie this answer set.
    fn session_cookie(&self) -> Option<String> {
        let set = self.header("set-cookie")?;
        let pair = set.split(';').next()?.trim();
        pair.starts_with(ui::COOKIE).then(|| pair.to_owned())
    }
}

struct Service {
    base: String,
    origin: String,
    admin: DatabaseConnection,
    db: DatabaseConnection,
    schema: String,
    scoped: String,
    hands: Hands,
    server: tokio::task::JoinHandle<()>,
}

impl Service {
    async fn start(what: &str, web: Option<&Path>) -> Self {
        Self::start_with(what, web, Vec::new()).await
    }

    async fn start_with(what: &str, web: Option<&Path>, proxies: Vec<std::net::IpAddr>) -> Self {
        let url = std::env::var("DATABASE_URL")
            .ok()
            .filter(|u| !u.is_empty())
            .expect(
                "DATABASE_URL is not set, and these tests are about PostgreSQL.\n  \
                 docker compose up -d\n  \
                 export DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee",
            );
        let schema: String = format!("fbui_{what}_{}", Uuid::now_v7().simple())
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
        let hands = Hands::new();
        let mut ui = Ui::default()
            .with_clock(hands.clock())
            .with_trusted_proxies(proxies);
        if let Some(dir) = web {
            ui = ui.with_web_dir(dir).expect("a web dir");
        }
        let config = baylee_feedback::Config::new(
            &format!("eu={GATEWAY},us={US}"),
            Some(READ),
            Some(ADMIN_TOKEN),
        )
        .expect("config");
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
            origin: format!("http://{addr}"),
            admin,
            db,
            schema,
            scoped,
            hands,
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

    /// The headers this UI's own pages send with a change.
    fn same_origin(&self) -> Vec<(&'static str, String)> {
        vec![
            ("origin", self.origin.clone()),
            (ui::CSRF_HEADER, "1".to_owned()),
        ]
    }

    /// As [`Self::same_origin`], signed in with `cookie`.
    fn signed_in(&self, cookie: &str) -> Vec<(&'static str, String)> {
        let mut headers = self.same_origin();
        headers.push(("cookie", cookie.to_owned()));
        headers
    }

    async fn login_as(&self, name: &str, password: &str, extra: &[(&str, String)]) -> Answer {
        let mut headers = self.same_origin();
        headers.extend(extra.iter().map(|(k, v)| (*k, v.clone())));
        self.call(
            "POST",
            "/ui/api/login",
            &headers,
            Some(serde_json::json!({ "name": name, "password": password }).to_string()),
        )
        .await
    }

    /// Signs in as the test's admin; the cookie to send.
    async fn login(&self) -> String {
        let answer = self.login_as(NAME, PASSWORD, &[]).await;
        assert_eq!(
            answer.status,
            200,
            "{}",
            String::from_utf8_lossy(&answer.body)
        );
        answer.session_cookie().expect("a session cookie")
    }

    async fn me(&self, cookie: &str) -> u16 {
        self.call("GET", "/ui/api/me", &[("cookie", cookie.to_owned())], None)
            .await
            .status
    }

    /// Hands a report in as gateway `token`; its id.
    async fn hand_in(&self, token: &str, report: serde_json::Value) -> String {
        let answer = self
            .call(
                "POST",
                "/intake/reports",
                &[("authorization", format!("Bearer {token}"))],
                Some(report.to_string()),
            )
            .await;
        assert_eq!(answer.status, 201);
        answer.json()["report_id"].as_str().unwrap().to_owned()
    }

    async fn sessions(&self) -> i64 {
        self.db
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT count(*) AS n FROM feedback_session",
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "n")
            .unwrap()
    }
}

fn report(kind: &str, reporter: &str, text: &str, record: bool) -> serde_json::Value {
    serde_json::json!({
        "gateway": { "name": "Baylee EU", "url": "https://eu.example", "version": "0.1.0" },
        "reporter": reporter,
        "kind": kind,
        "text": text,
        "game_id": "g1",
        "client": { "build": { "version": "0.1.0" } },
        "record": record.then(|| serde_json::json!({ "complete": false, "gzip_base64": "H4sIAAAAAAAA" })),
    })
}

// ------------------------------------------------------------------ admin CLI

fn cli(scoped: &str, args: &[&str], stdin: &str) -> (bool, String, String) {
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-feedback"))
        .args(args)
        .env("FEEDBACK_DATABASE_URL", scoped)
        .env_remove("RUST_LOG")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the binary runs");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// The admin commands run with the server's env file, `RUST_LOG=info`
/// included; its notices once buried the one line saying the password was
/// refused. Only that line may reach the terminal.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_admin_cli_says_only_what_happened_even_under_rust_log_info() {
    let service = Service::start("cli-quiet", None).await;
    let scoped = service.scoped.clone();
    let (ok, out, err) = tokio::task::spawn_blocking(move || {
        let child = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-feedback"))
            .args(["admin", "add", "shorty"])
            .env("FEEDBACK_DATABASE_URL", &scoped)
            .env("RUST_LOG", "info")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("the binary runs");
        child.stdin.as_ref().unwrap().write_all(b"short\n").unwrap();
        let out = child.wait_with_output().unwrap();
        (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    })
    .await
    .unwrap();
    assert!(!ok);
    assert!(out.is_empty(), "{out}");
    assert_eq!(err.lines().count(), 1, "one line, the refusal:\n{err}");
    assert!(err.contains("at least 12 characters"), "{err}");

    let scoped = service.scoped.clone();
    let (ok, out, _) = tokio::task::spawn_blocking(move || cli(&scoped, &["admin", "list"], ""))
        .await
        .unwrap();
    assert!(ok);
    assert!(!out.contains("shorty"), "{out}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_admin_cli_adds_lists_and_removes_admins_and_their_sessions() {
    let service = Service::start("cli", None).await;
    let scoped = service.scoped.clone();
    let run = move |args: &'static [&'static str], stdin: &'static str| {
        let scoped = scoped.clone();
        tokio::task::spawn_blocking(move || cli(&scoped, args, stdin))
    };

    let (ok, out, err) = run(&["admin", "add", "ops"], "second password here\n")
        .await
        .unwrap();
    assert!(ok, "{out}{err}");
    let (ok, _, err) = run(&["admin", "add", "ops"], "another password!\n")
        .await
        .unwrap();
    assert!(!ok && err.contains("already exists"), "{err}");
    let (ok, _, err) = run(&["admin", "add", "shorty"], "short\n").await.unwrap();
    assert!(!ok && err.contains("at least"), "{err}");
    let (ok, _, err) = run(&["admin", "add", "not a name"], "long enough password\n")
        .await
        .unwrap();
    assert!(!ok && err.contains("not an admin name"), "{err}");

    // The password is the line as typed, spaces and all, without its ending.
    let answer = service.login_as("ops", "second password here", &[]).await;
    assert_eq!(answer.status, 200);
    let cookie = answer.session_cookie().unwrap();

    let (ok, out, _) = run(&["admin", "list"], "").await.unwrap();
    assert!(ok);
    let names: Vec<&str> = out.lines().map(|l| l.split('\t').next().unwrap()).collect();
    assert_eq!(names, ["ops", NAME]);
    assert!(
        out.lines().next().unwrap().ends_with("1 session(s)"),
        "{out}"
    );
    assert!(
        !out.contains("argon2"),
        "the list never shows a hash:\n{out}"
    );

    let (ok, _, _) = run(&["admin", "remove", "ops"], "").await.unwrap();
    assert!(ok);
    assert_eq!(
        service.me(&cookie).await,
        401,
        "their session went with them"
    );
    let (ok, _, err) = run(&["admin", "remove", "ops"], "").await.unwrap();
    assert!(!ok && err.contains("no admin"), "{err}");
    let (ok, out, _) = run(&["admin", "list"], "").await.unwrap();
    assert!(ok);
    assert_eq!(out.lines().count(), 1, "{out}");
    let (ok, _, _) = run(&["admin", "frobnicate"], "").await.unwrap();
    assert!(!ok);

    service.close().await;
}

// ------------------------------------------------------------------ sign-in

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_wrong_password_and_an_unknown_name_get_the_same_answer() {
    let service = Service::start("login", None).await;
    let wrong = service.login_as(NAME, "not the password", &[]).await;
    let unknown = service.login_as("nobody", PASSWORD, &[]).await;
    let invalid = service.login_as("not a name!", PASSWORD, &[]).await;
    for answer in [&wrong, &unknown, &invalid] {
        assert_eq!(answer.status, 401);
        assert_eq!(answer.body, wrong.body);
        assert_eq!(answer.session_cookie(), None);
    }
    assert_eq!(wrong.json()["error"], "wrong name or password");
    assert_eq!(service.sessions().await, 0);

    let right = service.login_as(NAME, PASSWORD, &[]).await;
    assert_eq!(right.status, 200);
    assert_eq!(right.json()["name"], NAME);
    let set = right.header("set-cookie").unwrap();
    for part in ["HttpOnly", "Secure", "SameSite=Strict", "Path=/"] {
        assert!(set.contains(part), "{set}");
    }
    let cookie = right.session_cookie().unwrap();
    assert_eq!(service.me(&cookie).await, 200);
    // Kept as a hash: the token the cookie carries is nowhere in the table.
    let token = cookie.split_once('=').unwrap().1.to_owned();
    let stored: Vec<u8> = service
        .db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT token_hash FROM feedback_session",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "token_hash")
        .unwrap();
    assert_eq!(stored.len(), 32);
    assert_ne!(stored, token.as_bytes());
    assert_eq!(token.len(), 64);
    service.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sign_in_is_limited_per_name_and_per_address() {
    let proxy: std::net::IpAddr = "127.0.0.1".parse().unwrap();
    let service = Service::start_with("limit", None, vec![proxy]).await;
    let from = |address: &str| vec![("x-forwarded-for", address.to_owned())];

    // Five failures for one name, each from another address.
    for n in 0..ui::PER_NAME {
        let answer = service
            .login_as(NAME, "wrong", &from(&format!("10.0.0.{n}")))
            .await;
        assert_eq!(answer.status, 401, "failure {n}");
    }
    let refused = service.login_as(NAME, PASSWORD, &from("10.0.9.9")).await;
    assert_eq!(refused.status, 429, "even the right password waits");
    assert!(refused.header("retry-after").is_some());
    assert_eq!(refused.session_cookie(), None);

    // Ten failures from one address, each for another name.
    for n in 0..ui::PER_ADDRESS {
        let answer = service
            .login_as(&format!("guess{n}"), "wrong", &from("10.1.1.1"))
            .await;
        assert_eq!(answer.status, 401, "failure {n}");
    }
    assert_eq!(
        service
            .login_as("guess-new", "x", &from("10.1.1.1"))
            .await
            .status,
        429
    );
    assert_eq!(
        service
            .login_as("guess-new", "x", &from("10.1.1.2"))
            .await
            .status,
        401,
        "another address is not held"
    );

    service.hands.advance(ui::WINDOW);
    assert_eq!(
        service
            .login_as(NAME, PASSWORD, &from("10.1.1.1"))
            .await
            .status,
        200,
        "the window forgives both"
    );
    service.close().await;
}

// ------------------------------------------------------------------ sessions

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_session_lapses_when_idle_and_when_old() {
    let service = Service::start("expiry", None).await;
    let almost = ui::IDLE - Duration::minutes(1);

    // Idle: used just in time it lives on; left alone a full IDLE it ends.
    let cookie = service.login().await;
    service.hands.advance(almost);
    assert_eq!(service.me(&cookie).await, 200);
    service.hands.advance(almost);
    assert_eq!(
        service.me(&cookie).await,
        200,
        "each use moves the idle clock"
    );
    service.hands.advance(ui::IDLE);
    assert_eq!(service.me(&cookie).await, 401);
    assert_eq!(service.sessions().await, 0, "a lapsed session is removed");

    // Absolute: used every few hours, it still ends ABSOLUTE after sign-in.
    let cookie = service.login().await;
    let mut lived = Duration::ZERO;
    while lived + almost < ui::ABSOLUTE {
        service.hands.advance(almost);
        lived += almost;
        assert_eq!(service.me(&cookie).await, 200, "at {lived}");
    }
    service.hands.advance(ui::ABSOLUTE - lived);
    assert_eq!(service.me(&cookie).await, 401);
    service.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn signing_out_ends_the_session_on_the_server() {
    let service = Service::start("logout", None).await;
    let cookie = service.login().await;
    let other = service.login().await;
    let out = service
        .call("POST", "/ui/api/logout", &service.signed_in(&cookie), None)
        .await;
    assert_eq!(out.status, 204);
    assert!(out.header("set-cookie").unwrap().contains("Max-Age=0"));
    assert_eq!(
        service.me(&cookie).await,
        401,
        "the old cookie opens nothing"
    );
    assert_eq!(service.me(&other).await, 200, "another session is its own");
    assert_eq!(service.sessions().await, 1);
    service.close().await;
}

// ------------------------------------------------------------------ CSRF

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_change_without_the_header_or_from_another_origin_is_refused() {
    let service = Service::start("csrf", None).await;
    let cookie = service.login().await;
    let id = service
        .hand_in(GATEWAY, report("bug", "ab12", "text", false))
        .await;
    let path = format!("/ui/api/reports/{id}");
    let triage = Some(r#"{"status":"triaged"}"#.to_owned());
    let with_cookie = ("cookie", cookie.clone());
    let origin = ("origin", service.origin.clone());
    let csrf = (ui::CSRF_HEADER, "1".to_owned());
    let evil = ("origin", "https://evil.example".to_owned());
    for (why, headers) in [
        ("no header", vec![with_cookie.clone(), origin.clone()]),
        ("no origin", vec![with_cookie.clone(), csrf.clone()]),
        (
            "another origin",
            vec![with_cookie.clone(), csrf.clone(), evil.clone()],
        ),
        (
            "a cross-site fetch",
            vec![
                with_cookie.clone(),
                csrf.clone(),
                origin.clone(),
                ("sec-fetch-site", "cross-site".to_owned()),
            ],
        ),
    ] {
        for (method, body) in [("PATCH", triage.clone()), ("DELETE", None)] {
            let answer = service.call(method, &path, &headers, body).await;
            assert_eq!(answer.status, 403, "{method} {why}");
        }
        let out = service.call("POST", "/ui/api/logout", &headers, None).await;
        assert_eq!(out.status, 403, "logout {why}");
    }
    assert_eq!(
        service
            .call(
                "POST",
                "/ui/api/login",
                &[evil.clone(), csrf.clone()],
                Some(format!(r#"{{"name":"{NAME}","password":"{PASSWORD}"}}"#)),
            )
            .await
            .status,
        403,
        "a sign-in from another origin"
    );
    let still = service
        .call("GET", &path, std::slice::from_ref(&with_cookie), None)
        .await;
    assert_eq!(still.json()["status"], "new", "nothing changed");
    assert_eq!(service.me(&cookie).await, 200, "and nobody was signed out");
    service.close().await;
}

// ------------------------------------------------------------------ routes

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_ui_route_refuses_without_a_session() {
    let service = Service::start("refuse", None).await;
    let id = service
        .hand_in(GATEWAY, report("bug", "ab12", "text", true))
        .await;
    let routes = [
        ("GET", "/ui/api/me".to_owned(), None),
        ("GET", "/ui/api/facets".to_owned(), None),
        ("GET", "/ui/api/stats".to_owned(), None),
        ("GET", "/ui/api/reports".to_owned(), None),
        ("GET", format!("/ui/api/reports/{id}"), None),
        ("GET", format!("/ui/api/reports/{id}/record"), None),
        ("GET", format!("/ui/api/reports/{id}/audit"), None),
        (
            "PATCH",
            format!("/ui/api/reports/{id}"),
            Some(r#"{"status":"resolved"}"#.to_owned()),
        ),
        ("DELETE", format!("/ui/api/reports/{id}"), None),
        ("POST", "/ui/api/logout".to_owned(), None),
    ];
    let bogus = format!("{}={}", ui::COOKIE, "0".repeat(64));
    for (method, path, body) in &routes {
        for (why, credential) in [
            ("nothing", None),
            ("a made-up session", Some(("cookie", bogus.clone()))),
            (
                "the admin token",
                Some(("authorization", format!("Bearer {ADMIN_TOKEN}"))),
            ),
        ] {
            let mut headers = service.same_origin();
            headers.extend(credential);
            let answer = service.call(method, path, &headers, body.clone()).await;
            assert_eq!(answer.status, 401, "{method} {path} with {why}");
        }
    }
    // And the other way round: a session opens none of the token routes.
    let cookie = service.login().await;
    for path in ["/reports".to_owned(), format!("/reports/{id}")] {
        let answer = service
            .call("GET", &path, &[("cookie", cookie.clone())], None)
            .await;
        assert_eq!(answer.status, 401, "{path}");
    }
    let answer = service
        .call(
            "GET",
            "/reports",
            &[("authorization", format!("Bearer {READ}"))],
            None,
        )
        .await;
    assert_eq!(answer.status, 200, "the token API still reads");
    assert_eq!(
        answer.json()["reports"][0]["issue_number"],
        serde_json::Value::Null
    );
    service.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::too_many_lines)] // one catalogue of filters
async fn the_list_filters_and_pages_on_the_server() {
    let service = Service::start("filters", None).await;
    let cookie = service.login().await;
    let a = service
        .hand_in(
            GATEWAY,
            report("bug", "aaaa", "The swamp untapped itself", true),
        )
        .await;
    let b = service
        .hand_in(
            GATEWAY,
            report("crash", "aaaa", "crash at 100% load", false),
        )
        .await;
    let c = service
        .hand_in(US, report("improvement", "bbbb", "a better swamp", false))
        .await;
    let d = service
        .hand_in(
            US,
            report("bug", "cccc", "under_score and back\\slash", true),
        )
        .await;
    // Days apart, so a date range has something to cut.
    for (id, day) in [
        (&a, "2026-09-01"),
        (&b, "2026-09-10"),
        (&c, "2026-09-20"),
        (&d, "2026-09-27"),
    ] {
        service
            .db
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "UPDATE feedback_report SET created_at = ($2::date + time '12:00') AT TIME ZONE 'UTC' \
                 WHERE id = $1::uuid",
                [id.as_str().into(), (*day).into()],
            ))
            .await
            .unwrap();
    }
    let list = |query: &'static str| {
        let cookie = cookie.clone();
        let service = &service;
        async move {
            let answer = service
                .call(
                    "GET",
                    &format!("/ui/api/reports{query}"),
                    &[("cookie", cookie)],
                    None,
                )
                .await;
            (answer.status, answer.json())
        }
    };
    // Each listed report by its letter above.
    let letters = [(&a, "a"), (&b, "b"), (&c, "c"), (&d, "d")];
    let ids = |json: &serde_json::Value| -> Vec<&'static str> {
        json["reports"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                let id = r["id"].as_str().unwrap();
                letters.iter().find(|(i, _)| i.as_str() == id).unwrap().1
            })
            .collect()
    };

    let (_, all) = list("").await;
    assert_eq!(all["total"], 4);
    assert_eq!(ids(&all), ["d", "c", "b", "a"], "newest first");
    assert!(all["reports"][0].get("client").is_none());

    for (query, expected) in [
        ("?kind=bug", vec!["d", "a"]),
        ("?kind=improvement", vec!["c"]),
        ("?gateway=us", vec!["d", "c"]),
        ("?reporter=aaaa", vec!["b", "a"]),
        ("?has_record=true", vec!["d", "a"]),
        ("?has_record=false", vec!["c", "b"]),
        ("?q=SWAMP", vec!["c", "a"]),
        ("?q=100%25", vec!["b"]),
        ("?q=_", vec!["d"]),
        ("?q=%5C", vec!["d"]),
        ("?q=%25", vec!["b"]),
        ("?q=nothing+like+it", vec![]),
        ("?from=2026-09-10", vec!["d", "c", "b"]),
        ("?to=2026-09-10", vec!["b", "a"]),
        ("?from=2026-09-10&to=2026-09-20", vec!["c", "b"]),
        ("?from=2026-09-20&to=2026-09-20", vec!["c"]),
        ("?kind=bug&gateway=eu&has_record=true&q=swamp", vec!["a"]),
        ("?status=new", vec!["d", "c", "b", "a"]),
        ("?status=resolved", vec![]),
    ] {
        let (status, json) = list(query).await;
        assert_eq!(status, 200, "{query}");
        assert_eq!(ids(&json), expected, "{query}");
        assert_eq!(json["total"], expected.len(), "{query}");
    }

    // Pages: disjoint, in order, and the total counts all of them.
    let (_, first) = list("?limit=3").await;
    let (_, second) = list("?limit=3&offset=3").await;
    assert_eq!(first["total"], 4);
    assert_eq!(second["total"], 4);
    assert_eq!(ids(&first), ["d", "c", "b"]);
    assert_eq!(ids(&second), ["a"]);
    let (_, past) = list("?limit=3&offset=30").await;
    assert_eq!(ids(&past), Vec::<&str>::new());
    let (_, clamped) = list("?limit=0").await;
    assert_eq!(ids(&clamped).len(), 1, "a limit is at least one");

    for bad in [
        "?from=2026-9-1",
        "?from=2026-02-30",
        "?to=yesterday",
        "?kind=rant",
        "?has_record=maybe",
        "?limit=-1",
    ] {
        assert_eq!(list(bad).await.0, 400, "{bad}");
    }

    let facets = service
        .call("GET", "/ui/api/facets", &[("cookie", cookie.clone())], None)
        .await
        .json();
    assert_eq!(
        facets["reporters"][0],
        serde_json::json!({ "value": "aaaa", "count": 2 }),
        "the busiest pseudonym first"
    );
    assert_eq!(facets["gateways"].as_array().unwrap().len(), 2);
    assert_eq!(
        facets["statuses"],
        serde_json::json!([{ "value": "new", "count": 4 }])
    );

    // Per day and kind, the last 30 days only: a fresh bug and a fresh
    // crash land on today, by day then kind, after whichever of the four
    // above still fall in the window (none older than 30 days does).
    service
        .hand_in(GATEWAY, report("crash", "dddd", "it fell over", false))
        .await;
    service
        .hand_in(US, report("bug", "dddd", "it fell over again", false))
        .await;
    let stats = service
        .call("GET", "/ui/api/stats", &[("cookie", cookie.clone())], None)
        .await
        .json();
    assert_eq!(stats["days"], 30);
    let rows = stats["rows"].as_array().unwrap();
    let today = rows[rows.len() - 1]["day"].as_str().unwrap().to_owned();
    assert!(today.len() == 10 && today.starts_with("20"), "{stats}");
    let todays: Vec<(&str, i64)> = rows
        .iter()
        .filter(|r| r["day"] == today.as_str())
        .map(|r| (r["kind"].as_str().unwrap(), r["count"].as_i64().unwrap()))
        .collect();
    assert_eq!(todays, [("bug", 1), ("crash", 1)], "{stats}");
    assert!(
        rows.iter()
            .all(|r| r["day"].as_str().unwrap() >= "2026-09-10"),
        "a report older than the window: {stats}"
    );
    let days: Vec<&str> = rows.iter().map(|r| r["day"].as_str().unwrap()).collect();
    assert!(days.windows(2).all(|w| w[0] <= w[1]), "{stats}");
    service.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::too_many_lines)] // one report's life, change by change
async fn status_issue_and_deletion_are_changed_and_audited() {
    let service = Service::start("audit", None).await;
    let cookie = service.login().await;
    let id = service
        .hand_in(GATEWAY, report("bug", "ab12", "text", true))
        .await;
    let path = format!("/ui/api/reports/{id}");
    let patch = |body: &'static str| {
        let headers = service.signed_in(&cookie);
        let path = path.clone();
        let service = &service;
        async move {
            service
                .call("PATCH", &path, &headers, Some(body.to_owned()))
                .await
        }
    };

    let detail = service
        .call("GET", &path, &[("cookie", cookie.clone())], None)
        .await
        .json();
    assert_eq!(detail["client"]["build"]["version"], "0.1.0");
    assert_eq!(detail["has_record"], true);
    assert_eq!(detail["record_complete"], false);

    let changed = patch(r#"{"status":"triaged"}"#).await;
    assert_eq!(changed.status, 200);
    assert_eq!(changed.json()["status"], "triaged");
    let linked = patch(r#"{"issue":311}"#).await.json();
    assert_eq!(linked["issue_number"], 311);
    assert_eq!(
        linked["issue_url"],
        "https://github.com/AceVik/baylee/issues/311"
    );
    assert_eq!(
        linked["status"], "triaged",
        "an issue leaves the status alone"
    );
    let both = patch(r#"{"status":"in_progress","issue":312}"#)
        .await
        .json();
    assert_eq!(
        (both["status"].as_str(), both["issue_number"].as_i64()),
        (Some("in_progress"), Some(312))
    );
    let unlinked = patch(r#"{"issue":null}"#).await.json();
    assert_eq!(unlinked["issue_number"], serde_json::Value::Null);
    assert_eq!(unlinked["issue_url"], serde_json::Value::Null);
    for bad in [
        "{}",
        r#"{"issue":0}"#,
        r#"{"issue":-3}"#,
        r#"{"issue":"311"}"#,
        r#"{"issue":4294967296}"#,
        r#"{"issue":2147483648}"#,
        r#"{"status":"lost"}"#,
        r#"{"status":"new","owner":"me"}"#,
    ] {
        assert_eq!(patch(bad).await.status, 400, "{bad}");
    }
    // The token API writes the audit too, under `token`.
    let token = service
        .call(
            "PATCH",
            &format!("/reports/{id}"),
            &[("authorization", format!("Bearer {ADMIN_TOKEN}"))],
            Some(r#"{"status":"resolved"}"#.to_owned()),
        )
        .await;
    assert_eq!(token.status, 200);

    let record = service
        .call(
            "GET",
            &format!("{path}/record"),
            &[("cookie", cookie.clone())],
            None,
        )
        .await;
    assert_eq!(record.status, 200);
    assert_eq!(record.header("content-type"), Some("application/gzip"));
    assert!(
        record
            .header("content-disposition")
            .unwrap()
            .starts_with("attachment")
    );

    let deleted = service
        .call("DELETE", &path, &service.signed_in(&cookie), None)
        .await;
    assert_eq!(deleted.status, 204);
    assert_eq!(
        service
            .call("DELETE", &path, &service.signed_in(&cookie), None)
            .await
            .status,
        404
    );
    assert_eq!(
        service
            .call("GET", &path, &[("cookie", cookie.clone())], None)
            .await
            .status,
        404
    );
    let trail = service
        .call(
            "GET",
            &format!("{path}/audit"),
            &[("cookie", cookie.clone())],
            None,
        )
        .await
        .json();
    let steps: Vec<(String, String, Option<String>)> = trail
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e["actor"].as_str().unwrap().to_owned(),
                e["action"].as_str().unwrap().to_owned(),
                e["detail"].as_str().map(str::to_owned),
            )
        })
        .collect();
    let step = |actor: &str, action: &str, detail: Option<&str>| {
        (
            actor.to_owned(),
            action.to_owned(),
            detail.map(str::to_owned),
        )
    };
    assert_eq!(
        steps,
        [
            step(NAME, "status", Some("triaged")),
            step(NAME, "issue", Some("#311")),
            step(NAME, "status", Some("in_progress")),
            step(NAME, "issue", Some("#312")),
            step(NAME, "issue", None),
            step("token", "status", Some("resolved")),
            step(NAME, "delete", None),
        ],
        "the audit outlives the report"
    );
    service.close().await;
}

// ------------------------------------------------------------------ static files

/// A built UI in a scratch directory, and a secret beside it.
fn web_dir(what: &str) -> (PathBuf, PathBuf) {
    let scratch = std::env::temp_dir().join(format!(
        "baylee-feedback-web-{what}-{}",
        Uuid::now_v7().simple()
    ));
    let dist = scratch.join("dist");
    std::fs::create_dir_all(dist.join("assets")).unwrap();
    std::fs::write(
        dist.join("index.html"),
        "<!doctype html><title>t</title><script type=module src=/assets/app-abc123.js></script>",
    )
    .unwrap();
    std::fs::write(dist.join("assets/app-abc123.js"), "console.log(1)").unwrap();
    std::fs::write(dist.join("assets/app-abc123.css"), "body{}").unwrap();
    std::fs::write(dist.join("robots.txt"), "User-agent: *").unwrap();
    std::fs::write(scratch.join("secret.txt"), "not for the web").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(scratch.join("secret.txt"), dist.join("link.txt")).unwrap();
    (scratch, dist)
}

/// A request sent byte for byte, so a `..` reaches the server as written.
fn raw_get(base: &str, path: &str) -> u16 {
    let mut stream = std::net::TcpStream::connect(base.trim_start_matches("http://")).unwrap();
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut out = String::new();
    let _ = stream.read_to_string(&mut out);
    assert!(!out.contains("not for the web"), "{path} leaked the secret");
    out.split(' ')
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_ui_is_served_with_its_types_caching_and_headers() {
    let (scratch, dist) = web_dir("serve");
    let service = Service::start("static", Some(&dist)).await;
    let get = |path: &'static str| service.call("GET", path, &[], None);

    for path in ["/", "/index.html", "/r/0199-some-report", "/login"] {
        let page = get(path).await;
        assert_eq!(page.status, 200, "{path}");
        assert_eq!(
            page.header("content-type"),
            Some("text/html; charset=utf-8"),
            "{path}"
        );
        assert_eq!(page.header("cache-control"), Some("no-cache"), "{path}");
        assert!(String::from_utf8_lossy(&page.body).contains("app-abc123.js"));
    }
    let script = get("/assets/app-abc123.js").await;
    assert_eq!(script.status, 200);
    assert_eq!(
        script.header("content-type"),
        Some("text/javascript; charset=utf-8")
    );
    assert_eq!(
        script.header("cache-control"),
        Some("public, max-age=31536000, immutable")
    );
    assert_eq!(
        get("/assets/app-abc123.css").await.header("content-type"),
        Some("text/css; charset=utf-8")
    );
    assert_eq!(
        get("/robots.txt").await.header("cache-control"),
        Some("no-cache")
    );

    // Every answer carries the headers, a page, a file, a refusal and JSON.
    for path in [
        "/",
        "/assets/app-abc123.js",
        "/assets/gone.js",
        "/health",
        "/reports",
    ] {
        let answer = get(path).await;
        assert_eq!(
            answer.header("content-security-policy"),
            Some(baylee_feedback::web::CSP),
            "{path}"
        );
        assert_eq!(answer.header("x-frame-options"), Some("DENY"), "{path}");
        assert_eq!(
            answer.header("referrer-policy"),
            Some("no-referrer"),
            "{path}"
        );
        assert_eq!(
            answer.header("x-content-type-options"),
            Some("nosniff"),
            "{path}"
        );
    }

    assert_eq!(
        get("/assets/gone.js").await.status,
        404,
        "a missing file is not the page"
    );
    assert_eq!(get("/nothing.png").await.status, 404);
    let api_miss = get("/ui/api/nothing").await;
    assert_eq!(api_miss.status, 404);
    assert_eq!(api_miss.json()["error"], "no such route");
    assert_eq!(
        get("/reports").await.status,
        401,
        "the token API is untouched"
    );
    assert_eq!(get("/health").await.status, 200);
    assert_eq!(
        service
            .call("POST", "/", &[], Some("{}".into()))
            .await
            .status,
        405
    );

    let base = service.base.clone();
    let statuses = tokio::task::spawn_blocking(move || {
        [
            "/../secret.txt",
            "/assets/../../secret.txt",
            "/%2e%2e/secret.txt",
            "/%2e%2e%2fsecret.txt",
            "/..%5csecret.txt",
            "/link.txt",
            "/./index.html",
        ]
        .map(|p| (p, raw_get(&base, p)))
    })
    .await
    .unwrap();
    for (path, status) in statuses {
        assert!(matches!(status, 400 | 404), "{path}: {status}");
    }

    service.close().await;
    let _ = std::fs::remove_dir_all(scratch);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn without_a_web_dir_there_is_no_page() {
    let service = Service::start("noweb", None).await;
    for path in ["/", "/index.html", "/r/abc"] {
        assert_eq!(
            service.call("GET", path, &[], None).await.status,
            404,
            "{path}"
        );
    }
    service.close().await;
    let empty = std::env::temp_dir().join(format!("baylee-feedback-empty-{}", Uuid::now_v7()));
    std::fs::create_dir_all(&empty).unwrap();
    assert!(
        Ui::default().with_web_dir(&empty).is_err(),
        "a directory with no index.html refuses to start"
    );
    assert!(Ui::default().with_web_dir(empty.join("nope")).is_err());
    let _ = std::fs::remove_dir_all(empty);
}
