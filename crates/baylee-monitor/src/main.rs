//! Read-only workstation monitor. See docs/workstation-monitor.md.

mod gpu;
mod power;
mod sampler;
mod server;
mod types;

use clap::Parser;
use std::{
    collections::VecDeque,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::{Semaphore, watch};

#[derive(Parser)]
#[command(version, about = "Baylee Pulse · read-only workstation telemetry")]
struct Options {
    /// Explicit loopback or trusted LAN address; wildcard bindings are rejected.
    #[arg(long, default_value = "127.0.0.1:8787")]
    bind: SocketAddr,
    /// Sampling interval in milliseconds (500–10000).
    #[arg(long, default_value_t = 1000, value_parser = clap::value_parser!(u64).range(500..=10000))]
    interval_ms: u64,
    /// In-memory history in minutes (1–120).
    #[arg(long, default_value_t = 60, value_parser = clap::value_parser!(u64).range(1..=120))]
    history_minutes: u64,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let options = Options::parse();
    anyhow::ensure!(
        !options.bind.ip().is_unspecified(),
        "Use an explicit loopback or LAN address, not a wildcard"
    );
    let listener = tokio::net::TcpListener::bind(options.bind).await?;
    let interval = Duration::from_millis(options.interval_ms);
    let keep = (options.history_minutes * 60_000 / options.interval_ms) as usize;
    let mut sampler = tokio::task::spawn_blocking(sampler::Sampler::new).await?;
    let facts = sampler.facts(options.interval_ms);
    // First published CPU rates need two observations separated by an interval.
    tokio::time::sleep(interval).await;
    let (sender, receiver) = watch::channel(Arc::new(String::new()));
    let state = server::State {
        facts,
        history: Arc::new(Mutex::new(VecDeque::with_capacity(keep))),
        receiver,
        clients: Arc::new(Semaphore::new(12)),
        authority: options.bind.to_string(),
    };
    let history = state.history.clone();
    // Dedicated thread, no timer burst catch-up and no blocking work on the HTTP executor.
    std::thread::Builder::new()
        .name("telemetry".into())
        .spawn(move || {
            loop {
                let start = std::time::Instant::now();
                let snapshot = sampler.sample();
                {
                    let mut ring = history
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    if ring.len() == keep {
                        ring.pop_front();
                    }
                    ring.push_back(snapshot.point.clone());
                }
                match serde_json::to_string(
                    &serde_json::json!({"type": "sample", "snapshot": snapshot}),
                ) {
                    Ok(json) => {
                        sender.send_replace(Arc::new(json));
                    }
                    Err(error) => eprintln!("Telemetry serialization failed: {error}"),
                }
                std::thread::sleep(interval.saturating_sub(start.elapsed()));
            }
        })?;
    eprintln!(
        "Baylee Pulse: http://{} · {} ms · read only",
        options.bind, options.interval_ms
    );
    axum::serve(listener, server::router(state))
        .with_graceful_shutdown(shutdown())
        .await?;
    Ok(())
}

async fn shutdown() {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("SIGTERM handler");
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = term.recv() => {} }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}
