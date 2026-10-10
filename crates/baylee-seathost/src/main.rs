//! `baylee-seathost`: a seat agent (`docs/llm-seat.md` §"A hosted seat").
//!
//! ```text
//! BAYLEE_GATEWAY=unix:/run/baylee/gateway.sock \
//! BAYLEE_SEATHOST_TOKEN=… BAYLEE_SEATHOST_NAME=main \
//! BAYLEE_SEATHOST_STATE=/var/lib/baylee-llm/main \
//! BAYLEE_SEATHOST_BRIDGE_GATEWAY=http://127.0.0.1:28766 \
//! BAYLEE_KEY_STORE=file:/var/lib/baylee-llm/main/keys \
//!   baylee-seathost
//! ```
//!
//! `BAYLEE_SEATHOST_CAPACITY` bounds the bridges at once (0 = none);
//! `BAYLEE_SEAT_BIN` names the bridge (default: `baylee-seat` beside this
//! program). A release seat agent refuses a debug bridge, which would show
//! the model's reasoning to the whole table; a debug one says so and runs
//! it. On `SIGTERM` it takes no new chair and waits for its bridges'
//! games to end.

use baylee_seathost::launch::ProcessLauncher;
use baylee_seathost::{Config, Host};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

fn var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

fn fail(why: &str) -> ! {
    eprintln!("baylee-seathost: {why}");
    std::process::exit(2);
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let token = var("BAYLEE_SEATHOST_TOKEN").unwrap_or_else(|| {
        fail("BAYLEE_SEATHOST_TOKEN is not set: the gateway takes no seat agent without it")
    });
    let name = var("BAYLEE_SEATHOST_NAME").unwrap_or_else(|| "main".into());
    if !baylee_protocol::seathost::valid_name(&name) {
        fail("BAYLEE_SEATHOST_NAME is 1-64 of A-Z a-z 0-9 . - _");
    }
    let gateway = var("BAYLEE_GATEWAY")
        .unwrap_or_else(|| "http://127.0.0.1:28766".into())
        .trim_end_matches('/')
        .to_string();
    let capacity = var("BAYLEE_SEATHOST_CAPACITY").map_or(0, |v| {
        v.parse()
            .unwrap_or_else(|_| fail("BAYLEE_SEATHOST_CAPACITY is a number"))
    });
    let state_dir = var("BAYLEE_SEATHOST_STATE").map_or_else(
        || fail("BAYLEE_SEATHOST_STATE is not set: where its profiles and books are"),
        PathBuf::from,
    );
    let bridge = var("BAYLEE_SEAT_BIN").map_or_else(
        || {
            std::env::current_exe()
                .ok()
                .and_then(|exe| {
                    exe.parent()
                        .map(|dir| dir.join(format!("baylee-seat{}", std::env::consts::EXE_SUFFIX)))
                })
                .unwrap_or_else(|| {
                    fail("cannot find baylee-seat beside this program; set BAYLEE_SEAT_BIN")
                })
        },
        PathBuf::from,
    );
    let bridge_gateway = var("BAYLEE_SEATHOST_BRIDGE_GATEWAY");
    if baylee_protocol::unix_socket(&gateway).is_some() && bridge_gateway.is_none() {
        fail(
            "on the gateway's unix socket, BAYLEE_SEATHOST_BRIDGE_GATEWAY names the http:// address its bridges dial",
        );
    }
    refuse_a_debug_bridge(&bridge).await;
    let env = |name: &str| std::env::var(name).ok();
    let keys = key_store(&env);
    let launcher = ProcessLauncher::new(bridge.clone(), state_dir.join("run"));
    let host = Host::new(Config {
        name: name.clone(),
        gateway: gateway.clone(),
        token,
        capacity,
        state_dir,
        bridge_gateway,
        keys,
        launcher: Arc::new(launcher),
    })
    .unwrap_or_else(|why| fail(&why));
    tracing::info!(name, gateway, capacity, bridge = %bridge.display(), "baylee-seathost starting");
    let serving = tokio::spawn(Arc::clone(&host).serve());
    stopped().await;
    tracing::info!(
        live = host.live(),
        "stopping: no new chair; waiting for the games playing"
    );
    host.drain();
    while host.live() > 0 {
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    serving.abort();
}

/// The key store the bridges read (`BAYLEE_KEY_STORE`): `file:<dir>` on a
/// server; `off` keeps none.
fn key_store(
    env: &dyn Fn(&str) -> Option<String>,
) -> Arc<dyn baylee_client_core::llmseat::keys::KeyStore> {
    use baylee_client_core::llmseat::keys::{FILE_STORE_PREFIX, FileKeys, MemoryKeys, STORE_ENV};
    match env(STORE_ENV).as_deref().map(str::trim) {
        Some(value) if value.starts_with(FILE_STORE_PREFIX) => {
            Arc::new(FileKeys::new(&value[FILE_STORE_PREFIX.len()..]))
        }
        _ => Arc::new(MemoryKeys::unavailable(&format!(
            "the seat agent keeps keys only in a directory ({STORE_ENV}=file:<dir>)"
        ))),
    }
}

/// A debug bridge forwards the model's reasoning to every seat (`AiLog`):
/// a release seat agent does not run one.
async fn refuse_a_debug_bridge(bridge: &std::path::Path) {
    let out = tokio::process::Command::new(bridge)
        .arg("--version")
        .env_clear()
        .output()
        .await
        .unwrap_or_else(|e| fail(&format!("{}: {e}", bridge.display())));
    let version = String::from_utf8_lossy(&out.stdout);
    if version.contains("debug") {
        if cfg!(debug_assertions) {
            tracing::warn!(%version, "a debug bridge: it shows the model's reasoning to the table");
        } else {
            fail(
                "the bridge is a debug build, which shows the model's reasoning to the whole table; install a release build",
            );
        }
    }
}

async fn stopped() {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .unwrap_or_else(|e| fail(&format!("SIGTERM: {e}")));
        tokio::select! {
            _ = term.recv() => {}
            _ = tokio::signal::ctrl_c() => {}
        }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}
