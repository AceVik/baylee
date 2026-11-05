//! Read-only workstation monitor. See docs/workstation-monitor.md.

mod admission;
mod assets;
mod background;
mod gpu;
mod history;
mod listen;
mod network;
mod power;
mod processes;
mod procfs;
mod sampler;
mod sensors;
mod server;
mod storage;
#[cfg(test)]
mod test_support;
mod training;
mod types;

use clap::Parser;
use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex, PoisonError},
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
    /// Another exact name browsers may use for this service, as HOST:PORT
    /// (repeatable), e.g. `bigace.local:8787`. No wildcards.
    #[arg(long = "allow-host", value_name = "HOST:PORT", value_parser = admission::parse_allowed_host)]
    allow_hosts: Vec<String>,
    /// A `train_ops` pipeline directory to show in the training panel
    /// (repeatable, absolute, at most 8).
    #[arg(long = "pipeline", value_name = "DIR", value_parser = training::parse_pipeline_dir)]
    pipelines: Vec<PathBuf>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let options = Options::parse();
    anyhow::ensure!(
        !options.bind.ip().is_unspecified(),
        "Use an explicit loopback or LAN address, not a wildcard"
    );
    anyhow::ensure!(
        options.pipelines.len() <= training::MAX_PIPELINES,
        "At most {} --pipeline directories",
        training::MAX_PIPELINES
    );
    let listener = listen::bind(options.bind, listen::ADDRESS_WAIT).await?;
    let interval = Duration::from_millis(options.interval_ms);
    let keep = (options.history_minutes * 60_000 / options.interval_ms) as usize;
    let mut sampler =
        tokio::task::spawn_blocking(move || sampler::Sampler::new(interval)).await??;
    let facts = sampler.facts(options.interval_ms);
    // First published CPU rates need two observations separated by an interval.
    tokio::time::sleep(interval).await;
    let (sender, receiver) = watch::channel(Arc::new(String::new()));
    let state = server::State {
        facts,
        history: Arc::new(Mutex::new(history::History::new(keep))),
        receiver,
        clients: Arc::new(Semaphore::new(12)),
        admission: admission::Admission::new(options.bind, &options.allow_hosts),
    };
    let history = state.history.clone();
    let latest_training = Arc::new(Mutex::new(training::Training::default()));
    training::spawn(options.pipelines.clone(), latest_training.clone())?;
    // Dedicated thread, no timer burst catch-up and no blocking work on the HTTP executor.
    std::thread::Builder::new()
        .name("telemetry".into())
        .spawn(move || {
            loop {
                let start = std::time::Instant::now();
                let snapshot = sampler.sample();
                history
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(snapshot.point.clone());
                let json = {
                    let training = latest_training
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner);
                    serde_json::to_string(&types::Sample::new(&snapshot, &training))
                };
                match json {
                    Ok(json) => {
                        sender.send_replace(Arc::new(json));
                    }
                    Err(error) => eprintln!("Telemetry serialization failed: {error}"),
                }
                std::thread::sleep(interval.saturating_sub(start.elapsed()));
            }
        })?;
    eprintln!(
        "Baylee Pulse: {} · {} ms · {} pipeline(s) · read only",
        state.admission.urls().join(" · "),
        options.interval_ms,
        options.pipelines.len()
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
