//! `POST /client/reports` (`docs/feedback.md` §"Straight from a client")
//! against a real PostgreSQL, over real HTTP with the peer's address, behind
//! a trusted proxy whose `X-Forwarded-For` names the client.

use std::fmt::Write as _;
use std::io::Write as _;
use std::net::SocketAddr;
use std::sync::Arc;

use base64::Engine as _;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Statement};
use uuid::Uuid;

const GATEWAY: &str = "gateway-token-eu-0001";
const READ: &str = "read-token-000000001";
const DIRECT_KEY: &str = "direct-key-0000000001";
const DEVICE: &str = "0123456789abcdef0123456789abcdef";

struct Service {
    base: String,
    admin: DatabaseConnection,
    db: DatabaseConnection,
    schema: String,
    server: tokio::task::JoinHandle<()>,
}

impl Service {
    /// The service, taking direct reports when `direct`, believing the
    /// loopback proxy's `X-Forwarded-For`.
    async fn start(what: &str, direct: bool) -> Self {
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

    async fn close(self) {
        self.server.abort();
        drop(self.db);
        self.admin
            .execute_unprepared(&format!("DROP SCHEMA \"{}\" CASCADE", self.schema))
            .await
            .expect("dropping the schema");
    }

    /// One request, as a client at `from` behind the proxy; the status, the
    /// `x-record-origin` header and the body.
    async fn call(
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

    async fn direct(&self, from: &str, body: &serde_json::Value) -> u16 {
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

fn gz(text: &str) -> Vec<u8> {
    let mut out = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    out.write_all(text.as_bytes()).unwrap();
    out.finish().unwrap()
}

/// A record as a client's local game writes one: a header and an input.
fn record() -> Vec<u8> {
    gz(concat!(
        r#"{"kind":"header","record":1,"build":"0.1.0","preset":{},"hash":"00"}"#,
        "\n",
        r#"{"kind":"input","n":0,"at":0,"seat":0,"by":"seat","action":"Pass","hash":"01"}"#,
        "\n"
    ))
}

fn report(record: Option<&[u8]>) -> serde_json::Value {
    serde_json::json!({
        "kind": "bug",
        "text": "the house AI passed forever",
        "build": { "version": "0.1.0-beta.4", "commit": "deadbeef" },
        "device": DEVICE,
        "client": { "system": { "platform": "macos/aarch64" } },
        "record": record.map(|r| serde_json::json!({
            "complete": false,
            "gzip_base64": base64::engine::general_purpose::STANDARD.encode(r),
        })),
    })
}

/// Off until the service is given its key: nothing is taken, and the
/// answer says so.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn without_a_key_the_direct_route_takes_nothing() {
    let service = Service::start("direct_off", false).await;
    assert_eq!(service.direct("203.0.113.1", &report(None)).await, 503);
    service.close().await;
}

/// A direct report is kept apart and marked: its channel, a gateway no
/// token can name, the device's pseudonym and the record as the client's;
/// and the address it came from is nowhere in it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_direct_report_is_kept_marked_and_names_no_address() {
    let service = Service::start("direct_kept", true).await;
    let (status, _, created) = service
        .call(
            "POST",
            "/client/reports",
            None,
            "203.0.113.7",
            Some(report(Some(&record())).to_string()),
        )
        .await;
    assert_eq!(status, 201, "{created}");
    let id = created["report_id"].as_str().unwrap().to_owned();
    let (_, _, one) = service
        .call("GET", &format!("/reports/{id}"), Some(READ), "x", None)
        .await;
    assert_eq!(one["channel"], "direct");
    assert_eq!(one["gateway"], "(direct)");
    assert_eq!(one["record_origin"], "client");
    assert_eq!(one["gateway_version"], "0.1.0-beta.4 (deadbeef)");
    assert_eq!(one["has_record"], true);
    let reporter = one["reporter"].as_str().unwrap();
    assert_eq!(reporter.len(), 64);
    assert_ne!(reporter, DEVICE);
    let (status, origin, _) = service
        .call(
            "GET",
            &format!("/reports/{id}/record"),
            Some(READ),
            "x",
            None,
        )
        .await;
    assert_eq!((status, origin.as_deref()), (200, Some("client")));
    // The row, every column as text: no address, the device's id neither.
    let row = service
        .db
        .query_one_raw(Statement::from_string(
            service.db.get_database_backend(),
            "SELECT row_to_json(r)::text AS t FROM feedback_report r",
        ))
        .await
        .unwrap()
        .unwrap();
    let text: String = row.try_get("", "t").unwrap();
    for absent in ["203.0.113.7", "127.0.0.1", DEVICE] {
        assert!(!text.contains(absent), "{absent} is in the row");
    }
    // Apart from the gateways': listed under its channel only.
    let (_, _, direct) = service
        .call("GET", "/reports?channel=direct", Some(READ), "x", None)
        .await;
    let (_, _, gateway) = service
        .call("GET", "/reports?channel=gateway", Some(READ), "x", None)
        .await;
    assert_eq!(
        (direct["total"].as_i64(), gateway["total"].as_i64()),
        (Some(1), Some(0))
    );
    service.close().await;
}

/// Each bound refuses, and a malformed report is refused for what it is.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_direct_report_is_held_to_its_bounds() {
    let service = Service::start("direct_bounds", true).await;
    let mut n = 0_u32;
    // A fresh address each time, so only the bound under test answers.
    let mut from = || {
        n += 1;
        format!("198.51.100.{n}")
    };
    let mut big = report(None);
    big["client"]["filler"] =
        serde_json::Value::String("x".repeat(baylee_feedback::direct::MAX_CLIENT_BYTES));
    assert_eq!(service.direct(&from(), &big).await, 413);
    let mut long = report(None);
    long["text"] = serde_json::Value::String("é".repeat(20_001));
    assert_eq!(service.direct(&from(), &long).await, 413);
    let garbage = report(Some(&gz("not a record")));
    assert_eq!(service.direct(&from(), &garbage).await, 400);
    let mut huge = String::from(
        "{\"kind\":\"header\",\"record\":1,\"build\":\"x\",\"preset\":{},\"hash\":\"0\"}\n",
    );
    // Text that hardly compresses, so the stream is large and not only its
    // unpacked size.
    let mut noise = String::with_capacity(1_600_000);
    for i in 0..200_000_u64 {
        let _ = write!(noise, "{:08x}", i.wrapping_mul(0x9e37_79b9_7f4a_7c15));
    }
    while huge.len() <= baylee_feedback::direct::MAX_CLIENT_RECORD_BYTES * 2 {
        let _ = writeln!(
            huge,
            "{{\"kind\":\"chair\",\"n\":0,\"at\":0,\"seat\":0,\"change\":\"{noise}\"}}"
        );
    }
    let too_large = report(Some(&gz(&huge)));
    assert_eq!(service.direct(&from(), &too_large).await, 413);
    for (field, value) in [
        ("device", serde_json::json!("not-a-device")),
        ("device", serde_json::json!(DEVICE.to_uppercase())),
        ("client", serde_json::json!("a string")),
        ("kind", serde_json::json!("praise")),
        ("account", serde_json::json!("someone")),
    ] {
        let mut bad = report(None);
        bad[field] = value;
        assert_eq!(service.direct(&from(), &bad).await, 400, "{field}");
    }
    let mut empty = report(None);
    empty["text"] = serde_json::json!("  ");
    assert_eq!(service.direct(&from(), &empty).await, 400);
    service.close().await;
}

/// One address is held to its allowance, refused requests counted, and
/// another address is not held to the first one's.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn one_address_is_held_to_its_allowance() {
    let service = Service::start("direct_rate", true).await;
    let mut bad = report(None);
    bad["kind"] = serde_json::json!("praise");
    assert_eq!(service.direct("192.0.2.1", &bad).await, 400);
    for _ in 1..baylee_feedback::direct::PER_ADDRESS {
        assert_eq!(service.direct("192.0.2.1", &report(None)).await, 201);
    }
    assert_eq!(service.direct("192.0.2.1", &report(None)).await, 429);
    assert_eq!(service.direct("192.0.2.2", &report(None)).await, 201);
    service.close().await;
}

/// A gateway passing on a client's record of a local game: checked as the
/// direct route checks one, kept marked as the client's, its channel the
/// gateway's.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_gateway_passes_on_a_clients_record_marked_as_the_clients() {
    let service = Service::start("direct_intake", false).await;
    let intake = |record: &[u8]| {
        serde_json::json!({
            "gateway": { "name": null, "url": null, "version": "0.1.0" },
            "reporter": "ab12cd34",
            "kind": "bug",
            "text": "in a local game",
            "client": {},
            "record": {
                "complete": true,
                "gzip_base64": base64::engine::general_purpose::STANDARD.encode(record),
            },
            "record_origin": "client",
        })
        .to_string()
    };
    let (status, _, _) = service
        .call(
            "POST",
            "/intake/reports",
            Some(GATEWAY),
            "x",
            Some(intake(&gz("nope"))),
        )
        .await;
    assert_eq!(status, 400, "a client's record is checked");
    let (status, _, created) = service
        .call(
            "POST",
            "/intake/reports",
            Some(GATEWAY),
            "x",
            Some(intake(&record())),
        )
        .await;
    assert_eq!(status, 201);
    let id = created["report_id"].as_str().unwrap();
    let (_, _, one) = service
        .call("GET", &format!("/reports/{id}"), Some(READ), "x", None)
        .await;
    assert_eq!(one["channel"], "gateway");
    assert_eq!(one["gateway"], "eu");
    assert_eq!(one["record_origin"], "client");
    service.close().await;
}
