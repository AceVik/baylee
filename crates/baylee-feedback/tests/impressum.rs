//! `GET /site/impressum/challenge`, `POST /site/impressum` and the honeypot
//! `GET /site/impressum/kontakt` (`docs/feedback.md` §"The Impressum")
//! against a real PostgreSQL, over real HTTP: the cross-origin rules, what a
//! request looks like to the assessment, the honeypot's mark on an address,
//! and a full proof of work answered once.
//!
//! Each test runs in a schema of its own, as `contact.rs` does; without
//! `DATABASE_URL` they fail rather than skip. Only invented details are used.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use baylee_feedback::impressum::{self, BASE_DIFFICULTY, MAX_EXTRA, PARTS};
use baylee_feedback::ui::Ui;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use uuid::Uuid;

const GATEWAY: &str = "gateway-token-eu-0001";
const READ: &str = "read-token-000000001";
const ADMIN_TOKEN: &str = "admin-token-00000001";
const CONSOLE_TOKEN: &str = "the-gateways-admin-token-0123456789abcdef";
const KEY: &str = "an-impressum-key-of-more-than-32-characters";

/// The page the Impressum is served from.
const SITE: &str = "https://site.example";
const EVIL: &str = "https://evil.example";

const FIREFOX: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:130.0) Gecko/20100101 Firefox/130.0";
const HEADLESS: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) \
                        HeadlessChrome/130.0.0.0 Safari/537.36";

const CHALLENGE: &str = "/site/impressum/challenge";
const POST: &str = "/site/impressum";
const HONEYPOT: &str = "/site/impressum/kontakt";

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
    fn difficulty(&self) -> u64 {
        assert_eq!(self.status, 200, "{}", self.text());
        self.json()["difficulty"].as_u64().expect("a difficulty")
    }
}

struct Service {
    base: String,
    admin: DatabaseConnection,
    db: DatabaseConnection,
    schema: String,
    server: tokio::task::JoinHandle<()>,
    /// Seconds the service's clock is ahead of the real one.
    ahead: Arc<AtomicI64>,
    /// The directory holding the blob, when the Impressum is on.
    dir: Option<PathBuf>,
}

impl Service {
    /// The service believing the loopback proxy's `X-Forwarded-For`, taking
    /// contact messages from `SITE`, with the Impressum on or off.
    async fn start(what: &str, impressum_on: bool) -> Self {
        let url = std::env::var("DATABASE_URL")
            .ok()
            .filter(|u| !u.is_empty())
            .expect(
                "DATABASE_URL is not set, and these tests are about PostgreSQL.\n  \
                 docker compose up -d\n  \
                 export DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee",
            );
        let schema: String = format!("fbim_{what}_{}", Uuid::now_v7().simple())
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
            baylee_feedback::Config::new(&format!("eu={GATEWAY}"), Some(READ), Some(ADMIN_TOKEN))
                .and_then(|c| c.with_gateway_admin("http://127.0.0.1:1", CONSOLE_TOKEN))
                .and_then(|c| c.with_contact_origins(SITE))
                .expect("config");
        let mut dir = None;
        if impressum_on {
            let made = std::env::temp_dir().join(format!("baylee-impressum-{}", Uuid::now_v7()));
            std::fs::create_dir_all(&made).expect("a directory");
            let path = made.join("impressum.blob");
            std::fs::write(&path, impressum::decoy_blob(&impressum::decoy(&[7_u8; 32])))
                .expect("a blob");
            config = config
                .with_impressum(KEY, path.to_str().unwrap())
                .expect("the Impressum");
            dir = Some(made);
        }
        let ahead = Arc::new(AtomicI64::new(0));
        let clock_ahead = Arc::clone(&ahead);
        let ui = Ui::default()
            .with_trusted_proxies(vec!["127.0.0.1".parse().unwrap()])
            .with_clock(Arc::new(move || {
                OffsetDateTime::now_utc()
                    + time::Duration::seconds(clock_ahead.load(Ordering::SeqCst))
            }));
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
            ahead,
            dir,
        }
    }

    async fn close(self) {
        self.server.abort();
        drop(self.db);
        if let Some(dir) = &self.dir {
            let _ = std::fs::remove_dir_all(dir);
        }
        self.admin
            .execute_unprepared(&format!("DROP SCHEMA \"{}\" CASCADE", self.schema))
            .await
            .expect("dropping the schema");
    }

    /// Moves the service's clock `seconds` further ahead.
    fn advance(&self, seconds: i64) {
        self.ahead.fetch_add(seconds, Ordering::SeqCst);
    }

    fn blob_file(&self) -> PathBuf {
        self.dir.as_ref().unwrap().join("impressum.blob")
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

    /// A challenge as a browser on the site's own origin asks for it, from
    /// the client address `from`.
    async fn browser_challenge(&self, from: &str) -> Answer {
        self.call(
            "GET",
            CHALLENGE,
            &[
                ("x-forwarded-for", from.to_owned()),
                ("sec-fetch-site", "same-origin".to_owned()),
                ("accept-language", "de,en;q=0.8".to_owned()),
                ("user-agent", FIREFOX.to_owned()),
            ],
            None,
        )
        .await
    }

    /// The honeypot link followed by `from`.
    async fn follow_honeypot(&self, from: &str) -> Answer {
        self.call(
            "GET",
            HONEYPOT,
            &[
                ("x-forwarded-for", from.to_owned()),
                ("user-agent", "ExampleBot/1.0".to_owned()),
            ],
            None,
        )
        .await
    }

    /// The page posts `body` from its own origin as a client at `from`.
    async fn browser_post(&self, from: &str, body: String) -> Answer {
        self.call(
            "POST",
            POST,
            &[
                ("x-forwarded-for", from.to_owned()),
                ("sec-fetch-site", "same-origin".to_owned()),
            ],
            Some(body),
        )
        .await
    }
}

/// The answer to `challenge` with these counters.
fn solution(challenge: &serde_json::Value, counters: &[u64]) -> serde_json::Value {
    serde_json::json!({
        "nonce": challenge["nonce"],
        "parts": challenge["parts"],
        "difficulty": challenge["difficulty"],
        "expires": challenge["expires"],
        "sig": challenge["sig"],
        "counters": counters,
    })
}

/// The smallest counter for every part, with the prefix's hash state kept
/// instead of formatting the whole message each time. The parts are
/// independent, so each gets a thread: a debug build hashes slowly.
fn solve(challenge: &serde_json::Value) -> Vec<u64> {
    let nonce = challenge["nonce"].as_str().unwrap();
    let parts = challenge["parts"].as_u64().unwrap();
    let difficulty = u32::try_from(challenge["difficulty"].as_u64().unwrap()).unwrap();
    std::thread::scope(|scope| {
        let threads: Vec<_> = (0..parts)
            .map(|part| {
                scope.spawn(move || {
                    let prefix = Sha256::new().chain_update(format!("{nonce}:{part}:"));
                    (0_u64..u64::MAX)
                        .find(|counter| {
                            let mut hasher = prefix.clone();
                            hasher.update(counter.to_string());
                            impressum::leading_zeros(&hasher.finalize(), difficulty)
                        })
                        .unwrap()
                })
            })
            .collect();
        threads.into_iter().map(|t| t.join().unwrap()).collect()
    })
}

fn assert_never_cached_or_indexed(answer: &Answer) {
    assert_eq!(answer.header("cache-control"), Some("no-store"));
    assert!(
        answer
            .header("x-robots-tag")
            .is_some_and(|v| v.contains("noindex")),
        "{:?}",
        answer.header("x-robots-tag")
    );
}

// ------------------------------------------------------------------ off

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn without_a_configured_impressum_the_challenge_and_the_answer_are_503() {
    let service = Service::start("off", false).await;
    let challenge = service.browser_challenge("203.0.113.1").await;
    assert_eq!(challenge.status, 503, "{}", challenge.text());
    assert_never_cached_or_indexed(&challenge);
    let post = service
        .browser_post(
            "203.0.113.1",
            solution(&serde_json::json!({}), &[]).to_string(),
        )
        .await;
    assert_eq!(post.status, 503, "{}", post.text());
    assert_never_cached_or_indexed(&post);
    // The honeypot is a trap and answers regardless, with an invented page.
    let trap = service.follow_honeypot("203.0.113.1").await;
    assert_eq!(trap.status, 200);
    assert!(trap.text().contains("@example."), "{}", trap.text());
    service.close().await;
}

// ------------------------------------------------------------------ the challenge

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_ordinary_browser_gets_the_base_puzzle() {
    let service = Service::start("base", true).await;
    let answer = service.browser_challenge("203.0.113.1").await;
    assert_eq!(answer.status, 200, "{}", answer.text());
    let c = answer.json();
    assert_eq!(c["difficulty"], BASE_DIFFICULTY);
    assert_eq!(c["parts"], PARTS);
    assert_eq!(
        c["nonce"].as_str().unwrap().len(),
        22,
        "16 bytes, base64url"
    );
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let expires = c["expires"].as_i64().unwrap();
    assert!(
        (now + 4 * 60..=now + 5 * 60 + 5).contains(&expires),
        "{expires} vs {now}"
    );
    assert!(!c["sig"].as_str().unwrap().is_empty());
    assert_never_cached_or_indexed(&answer);
    assert_eq!(answer.header("vary"), Some("Origin"));
    assert_eq!(
        answer.header("access-control-allow-origin"),
        None,
        "same-origin needs no CORS"
    );
    // Another request is another puzzle.
    let again = service.browser_challenge("203.0.113.1").await.json();
    assert_ne!(again["nonce"], c["nonce"]);
    assert_ne!(again["sig"], c["sig"]);
    service.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn another_site_is_refused_and_not_told_it_may_read() {
    let service = Service::start("evil", true).await;
    for (what, headers) in [
        (
            "cross-site with an evil origin",
            vec![
                ("origin", EVIL.to_owned()),
                ("sec-fetch-site", "cross-site".to_owned()),
            ],
        ),
        (
            "an evil origin saying nothing else",
            vec![("origin", EVIL.to_owned())],
        ),
        (
            "same-site, not the page's origin",
            vec![
                ("origin", EVIL.to_owned()),
                ("sec-fetch-site", "same-site".to_owned()),
            ],
        ),
        (
            "cross-site without an origin (an embedded fetch)",
            vec![("sec-fetch-site", "cross-site".to_owned())],
        ),
    ] {
        let answer = service.call("GET", CHALLENGE, &headers, None).await;
        assert_eq!(answer.status, 403, "{what}: {}", answer.text());
        assert_eq!(answer.header("access-control-allow-origin"), None, "{what}");
        assert_never_cached_or_indexed(&answer);
        let post = service
            .call("POST", POST, &headers, Some("{}".to_owned()))
            .await;
        assert_eq!(post.status, 403, "{what}: {}", post.text());
        assert_eq!(post.header("access-control-allow-origin"), None, "{what}");
    }
    // The preflight of an evil page is refused too.
    let preflight = service
        .call(
            "OPTIONS",
            POST,
            &[
                ("origin", EVIL.to_owned()),
                ("access-control-request-method", "POST".to_owned()),
            ],
            None,
        )
        .await;
    assert_eq!(preflight.status, 403);
    assert_eq!(preflight.header("access-control-allow-origin"), None);
    service.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_pages_own_origin_may_ask_and_read_the_answer() {
    let service = Service::start("site", true).await;
    let answer = service
        .call(
            "GET",
            CHALLENGE,
            &[
                ("origin", SITE.to_owned()),
                ("sec-fetch-site", "cross-site".to_owned()),
                ("accept-language", "de".to_owned()),
                ("user-agent", FIREFOX.to_owned()),
            ],
            None,
        )
        .await;
    assert_eq!(answer.status, 200, "{}", answer.text());
    assert_eq!(answer.header("access-control-allow-origin"), Some(SITE));
    assert_eq!(answer.header("vary"), Some("Origin"));
    assert_eq!(answer.json()["difficulty"], BASE_DIFFICULTY);
    // The page's POST is preflighted for application/json.
    let preflight = service
        .call(
            "OPTIONS",
            POST,
            &[
                ("origin", SITE.to_owned()),
                ("access-control-request-method", "POST".to_owned()),
                ("access-control-request-headers", "content-type".to_owned()),
            ],
            None,
        )
        .await;
    assert_eq!(preflight.status, 204);
    assert_eq!(preflight.header("access-control-allow-origin"), Some(SITE));
    assert!(
        preflight
            .header("access-control-allow-methods")
            .is_some_and(|m| m.contains("POST")),
        "{:?}",
        preflight.header("access-control-allow-methods")
    );
    // A browser too old to send either header is answered, and weighed.
    let old = service
        .call(
            "GET",
            CHALLENGE,
            &[
                ("accept-language", "de".to_owned()),
                ("user-agent", FIREFOX.to_owned()),
            ],
            None,
        )
        .await;
    assert_eq!(old.status, 200);
    assert_eq!(old.header("access-control-allow-origin"), None);
    assert_eq!(old.difficulty(), u64::from(BASE_DIFFICULTY) + 1);
    service.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_headless_client_gets_a_harder_puzzle_but_is_never_refused() {
    let service = Service::start("headless", true).await;
    let hard = service
        .call(
            "GET",
            CHALLENGE,
            &[("user-agent", HEADLESS.to_owned())],
            None,
        )
        .await;
    assert_eq!(hard.status, 200, "{}", hard.text());
    let difficulty = hard.difficulty();
    assert!(difficulty > u64::from(BASE_DIFFICULTY));
    assert!(difficulty <= u64::from(BASE_DIFFICULTY + MAX_EXTRA));
    // Nothing at all: no user agent, no language, no fetch metadata.
    let bare = service.call("GET", CHALLENGE, &[], None).await;
    assert_eq!(bare.status, 200, "{}", bare.text());
    assert!(bare.difficulty() > u64::from(BASE_DIFFICULTY));
    assert!(bare.difficulty() <= u64::from(BASE_DIFFICULTY + MAX_EXTRA));
    // What the page saw of its own browser adds to it.
    let driven = service
        .call(
            "GET",
            &format!("{CHALLENGE}?s=1"),
            &[
                ("sec-fetch-site", "same-origin".to_owned()),
                ("accept-language", "de".to_owned()),
                ("user-agent", FIREFOX.to_owned()),
            ],
            None,
        )
        .await;
    assert_eq!(driven.difficulty(), u64::from(BASE_DIFFICULTY) + 2);
    // Garbage in `s` says nothing; unknown bits are dropped.
    for query in ["?s=abc", "?s=", "?s=8", "?x=1"] {
        let quiet = service
            .call(
                "GET",
                &format!("{CHALLENGE}{query}"),
                &[
                    ("sec-fetch-site", "same-origin".to_owned()),
                    ("accept-language", "de".to_owned()),
                    ("user-agent", FIREFOX.to_owned()),
                ],
                None,
            )
            .await;
        assert_eq!(quiet.difficulty(), u64::from(BASE_DIFFICULTY), "{query}");
    }
    service.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn asking_again_and_again_raises_the_difficulty_for_that_address_only() {
    let service = Service::start("many", true).await;
    let mut seen = Vec::new();
    for _ in 0..101 {
        seen.push(service.browser_challenge("203.0.113.5").await.difficulty());
    }
    let base = u64::from(BASE_DIFFICULTY);
    assert!(seen[..10].iter().all(|d| *d == base), "{seen:?}");
    assert_eq!(seen[10], base + 1, "the 11th");
    assert_eq!(seen[29], base + 1, "the 30th");
    assert_eq!(seen[30], base + 2, "the 31st");
    assert_eq!(seen[99], base + 2, "the 100th");
    assert_eq!(seen[100], base + 3, "the 101st");
    // Never refused, and another address starts over.
    assert_eq!(
        service.browser_challenge("203.0.113.6").await.difficulty(),
        base
    );
    // After the hour it is forgotten.
    service.advance(3601);
    assert_eq!(
        service.browser_challenge("203.0.113.5").await.difficulty(),
        base
    );
    service.close().await;
}

// ------------------------------------------------------------------ the honeypot

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_honeypot_answers_an_invented_impressum_nobody_should_index() {
    let service = Service::start("trap", true).await;
    let answer = service.follow_honeypot("203.0.113.7").await;
    assert_eq!(answer.status, 200);
    assert!(
        answer
            .header("content-type")
            .is_some_and(|t| t.starts_with("text/html")),
        "{:?}",
        answer.header("content-type")
    );
    assert!(
        answer
            .header("x-robots-tag")
            .is_some_and(|v| v.contains("noindex")),
        "{:?}",
        answer.header("x-robots-tag")
    );
    let page = answer.text();
    assert!(page.contains("Impressum"), "{page}");
    assert!(page.contains("Deutschland"), "{page}");
    assert!(page.contains("@example."), "{page}");
    assert!(page.contains("+49 "), "{page}");
    assert!(page.contains("noindex"), "{page}");
    // The same address is shown the same invented person; another is not.
    let again = service.follow_honeypot("203.0.113.7").await.text();
    assert_eq!(again, page);
    let other = service.follow_honeypot("203.0.113.8").await.text();
    assert_ne!(other, page);
    service.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_address_that_asked_the_honeypot_gets_the_hardest_puzzle_for_a_day() {
    let service = Service::start("flag", true).await;
    let base = u64::from(BASE_DIFFICULTY);
    assert_eq!(
        service.browser_challenge("203.0.113.10").await.difficulty(),
        base
    );
    assert_eq!(service.follow_honeypot("203.0.113.10").await.status, 200);
    let hardest = u64::from(BASE_DIFFICULTY + MAX_EXTRA);
    let flagged = service.browser_challenge("203.0.113.10").await;
    assert_eq!(flagged.status, 200, "never refused: {}", flagged.text());
    assert_eq!(flagged.difficulty(), hardest);
    // Another address is not touched.
    assert_eq!(
        service.browser_challenge("203.0.113.11").await.difficulty(),
        base
    );
    // The mark lasts the day and then lets go.
    service.advance(23 * 3600);
    assert_eq!(
        service.browser_challenge("203.0.113.10").await.difficulty(),
        hardest
    );
    service.advance(2 * 3600);
    assert_eq!(
        service.browser_challenge("203.0.113.10").await.difficulty(),
        base
    );
    service.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_ipv6_subscriber_shares_one_mark_across_its_sixty_four() {
    let service = Service::start("v6", true).await;
    let base = u64::from(BASE_DIFFICULTY);
    service.follow_honeypot("2001:db8:1:2::1").await;
    assert_eq!(
        service
            .browser_challenge("2001:db8:1:2:aaaa:bbbb:cccc:dddd")
            .await
            .difficulty(),
        u64::from(BASE_DIFFICULTY + MAX_EXTRA)
    );
    assert_eq!(
        service
            .browser_challenge("2001:db8:1:3::1")
            .await
            .difficulty(),
        base
    );
    service.close().await;
}

// ------------------------------------------------------------------ the answer

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_body_that_is_no_solution_is_400_and_a_forged_challenge_403() {
    let service = Service::start("bad", true).await;
    let from = "203.0.113.20";
    for (what, body) in [
        ("not JSON", "this is not json".to_owned()),
        ("empty", String::new()),
        ("an empty object", "{}".to_owned()),
        ("a list", "[1,2,3]".to_owned()),
        ("a field too many", {
            let c = service.browser_challenge(from).await.json();
            let mut s = solution(&c, &[0; 8]);
            s["extra"] = serde_json::json!(1);
            s.to_string()
        }),
        ("counters that are no numbers", {
            let c = service.browser_challenge(from).await.json();
            let mut s = solution(&c, &[]);
            s["counters"] = serde_json::json!(["a", "b"]);
            s.to_string()
        }),
        (
            "past four kibibytes",
            format!("{{\"pad\":\"{}\"}}", "a".repeat(5000)),
        ),
    ] {
        let answer = service.browser_post(from, body).await;
        assert_eq!(answer.status, 400, "{what}: {}", answer.text());
        assert_never_cached_or_indexed(&answer);
    }

    // A challenge nobody here signed.
    let c = service.browser_challenge(from).await.json();
    let mut forged = solution(&c, &[0; 8]);
    let sig = c["sig"].as_str().unwrap();
    let flipped = if sig.starts_with('A') { 'B' } else { 'A' };
    forged["sig"] = serde_json::json!(format!("{flipped}{}", &sig[1..]));
    let answer = service.browser_post(from, forged.to_string()).await;
    assert_eq!(answer.status, 403, "{}", answer.text());
    assert_never_cached_or_indexed(&answer);
    // Or an easier puzzle under a signature for a harder one.
    let mut easier = solution(&c, &[0; 8]);
    easier["difficulty"] = serde_json::json!(1);
    let answer = service.browser_post(from, easier.to_string()).await;
    assert_eq!(answer.status, 403, "{}", answer.text());
    // Wrong counters under a good signature: refused, and not used up.
    let wrong = service
        .browser_post(from, solution(&c, &[0; 8]).to_string())
        .await;
    assert_eq!(wrong.status, 400, "{}", wrong.text());
    let short = service
        .browser_post(from, solution(&c, &[1, 2, 3]).to_string())
        .await;
    assert_eq!(short.status, 400, "{}", short.text());
    service.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_challenge_not_answered_in_time_is_gone() {
    let service = Service::start("late", true).await;
    let c = service.browser_challenge("203.0.113.30").await.json();
    service.advance(6 * 60);
    let answer = service
        .browser_post("203.0.113.30", solution(&c, &[0; 8]).to_string())
        .await;
    assert_eq!(answer.status, 410, "{}", answer.text());
    assert_never_cached_or_indexed(&answer);
    service.close().await;
}

/// A whole proof of work, 8 parts of 17 bits, solved in the test: the blob
/// comes back once, read from the file as it is now, and a second try with
/// the same answer is `409`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_solved_puzzle_gets_the_blob_once() {
    let service = Service::start("solve", true).await;
    let from = "203.0.113.40";
    let c = service.browser_challenge(from).await.json();
    assert_eq!(c["difficulty"], BASE_DIFFICULTY);
    let started = std::time::Instant::now();
    let counters = {
        let c = c.clone();
        tokio::task::spawn_blocking(move || solve(&c))
            .await
            .unwrap()
    };
    eprintln!(
        "solved {PARTS} x {BASE_DIFFICULTY} bits in {:?}",
        started.elapsed()
    );
    assert_eq!(counters.len(), PARTS as usize);

    // The details are read when asked: a file changed since the challenge is
    // the one served.
    let blob = impressum::decoy_blob(&impressum::decoy(&[9_u8; 32]));
    std::fs::write(service.blob_file(), format!("{blob}\n")).unwrap();

    let body = solution(&c, &counters).to_string();
    let answer = service.browser_post(from, body.clone()).await;
    assert_eq!(answer.status, 200, "{}", answer.text());
    assert_eq!(
        answer.header("content-type"),
        Some("text/plain; charset=us-ascii")
    );
    assert_never_cached_or_indexed(&answer);
    assert!(impressum::looks_like_blob(&answer.body));
    assert_eq!(answer.text(), blob, "the file's blob, without its newline");

    let again = service.browser_post(from, body).await;
    assert_eq!(again.status, 409, "{}", again.text());
    assert_never_cached_or_indexed(&again);

    // A second challenge is a second blob, whoever asks.
    let c2 = service.browser_challenge("203.0.113.41").await.json();
    assert_ne!(c2["nonce"], c["nonce"]);
    service.close().await;
}
