//! Bounded, synthetic local demo over the real authenticated transports.
//! No development shortcut reaches a production server or skips authentication.
#![forbid(unsafe_code)]

use super::{DevelopmentServer, Served, common, fresh};
use central_api::challenges as api;
use std::{
    net::TcpStream,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{runtime::Runtime, sync::oneshot};

pub(super) fn document(origin: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "version": 1, "gateways": [origin], "preferences": {}, "legacy": null,
        "chat": {"devices": [], "installations": []}
    }))
    .unwrap()
}

pub(super) fn run(runtime: &Runtime, served: &Served) {
    let path =
        PathBuf::from(std::env::var_os("BAYLEE_CENTRAL_UI_READY").expect("demo readiness path"));
    let site = std::env::var("BAYLEE_CENTRAL_DEMO_WEB_ORIGIN")
        .unwrap_or_else(|_| "http://127.0.0.1:28793".into());
    let site = central_http::BrowserOrigin::development(&site).expect("numeric loopback site");
    let server = runtime
        .block_on(DevelopmentServer::bind(
            central_policy::config::Config::from_pairs([
                ("BAYLEE_CENTRAL_LISTEN", "127.0.0.1:0"),
                ("BAYLEE_CENTRAL_REQUEST_BYTES", "65536"),
            ])
            .unwrap(),
        ))
        .unwrap();
    let address = server.local_addr();
    let (stop, stopped) = oneshot::channel();
    let identity = Arc::clone(&served.service);
    let origin = site.origin().to_owned();
    let browser = runtime.spawn(async move {
        server
            .run_browser(identity, site, async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    let agent = runtime.block_on(common::attach_agent(&served.gateway));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        prepare(runtime, served);
        std::fs::write(
            &path,
            serde_json::to_vec(&serde_json::json!({
                "mode": "synthetic-only", "central": served.endpoint.origin(),
                "gateway": served.origin, "browser": address.to_string(), "site": origin,
                "accounts": {
                    "new": "demo_player", "completed": "fresh_player", "resume": "demo_resume"
                }
            }))
            .unwrap(),
        )
        .unwrap();
        let seconds = std::env::var("BAYLEE_CENTRAL_UI_HOLD_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(3600)
            .min(7200);
        let deadline = Instant::now() + Duration::from_secs(seconds);
        while Instant::now() < deadline && !path.with_extension("stop").exists() {
            std::thread::sleep(Duration::from_millis(200));
        }
        std::fs::remove_file(&path).unwrap();
    }));
    agent.abort();
    let _ = runtime.block_on(agent);
    let _ = stop.send(());
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(5), browser)
            .await
            .unwrap()
            .unwrap();
    });
    assert_eq!(
        TcpStream::connect_timeout(&address, Duration::from_secs(5))
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::ConnectionRefused
    );
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

fn prepare(runtime: &Runtime, served: &Served) {
    let document = document(&served.origin);
    let new = fresh::register(
        runtime,
        &served.service,
        &served.endpoint,
        "demo_player",
        &document,
    );
    super::challenges::tower::prepare_demo(runtime, &served.gateway, &new);
    let returning = fresh::register(
        runtime,
        &served.service,
        &served.endpoint,
        "demo_resume",
        &document,
    );
    let api::Response::Catalogue { challenges, .. } = returning
        .challenges(api::Request::Catalogue { before: None })
        .unwrap()
    else {
        panic!("demo catalogue")
    };
    let now = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap();
    let task = challenges
        .iter()
        .find(|task| {
            task.description.period.kind == api::Kind::Weekly
                && task.description.period.live_at(now)
        })
        .expect("the current certified Weekly");
    let api::Response::Attempt { attempt } = returning
        .challenges(api::Request::Begin {
            challenge: task.id,
            request_id: *uuid::Uuid::now_v7().as_bytes(),
            listed: false,
        })
        .unwrap()
    else {
        panic!("demo reservation")
    };
    returning
        .play_challenge(attempt.id)
        .expect("the normal authenticated launch");
}
