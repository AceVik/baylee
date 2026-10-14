//! Bounded loopback-only recorder requests, with no proxies or redirects.
use super::{cli::Preview, data, render};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::path::Path;
use std::sync::{
    OnceLock,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

static CANCELLED: AtomicBool = AtomicBool::new(false);
static HANDLER: OnceLock<Result<(), String>> = OnceLock::new();

pub(super) fn request(port: u16, endpoint: &str, body: Option<&Value>) -> Result<Value> {
    ensure!(
        port != 0 && matches!(endpoint, "start" | "status" | "stop"),
        "invalid local recorder endpoint"
    );
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .proxy(None)
        .max_redirects(0)
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .into();
    let url = format!("http://127.0.0.1:{port}/recording/{endpoint}");
    let mut response = if let Some(body) = body {
        // A byte body gives ureq a fixed length; dev-control rejects chunking.
        let bytes = serde_json::to_vec(body)?;
        ensure!(bytes.len() <= 1024, "recording body exceeds 1 KiB");
        agent
            .post(&url)
            .header("content-type", "application/json")
            .send(bytes)?
    } else {
        agent.get(&url).call()?
    };
    ensure!(
        response.status().is_success(),
        "dev-control HTTP {}; redirects are refused",
        response.status()
    );
    let bytes = response
        .body_mut()
        .with_config()
        .limit(1024 * 1024)
        .read_to_vec()
        .context("status response exceeds 1 MiB or could not be read")?;
    let result: Value = serde_json::from_slice(&bytes)?;
    if let Some(error) = result.get("error") {
        bail!("dev-control: {error}");
    }
    Ok(result)
}

pub(super) fn record(port: u16, duration_ms: u32, fps: u32, preview: &Preview) -> Result<Value> {
    ensure!(
        (1..=30_000).contains(&duration_ms)
            && (1..=60).contains(&fps)
            && duration_ms * fps <= 900_000,
        "duration_ms must be 1..30000, fps 1..60, at most 900 nominal samples"
    );
    let handler = HANDLER.get_or_init(|| {
        ctrlc::set_handler(|| CANCELLED.store(true, Ordering::Relaxed))
            .map_err(|error| error.to_string())
    });
    if let Err(error) = handler {
        bail!("cannot install recording cancellation handler: {error}");
    }
    CANCELLED.store(false, Ordering::Relaxed);
    let started = request(
        port,
        "start",
        Some(&json!({"duration_ms":duration_ms,"fps":fps})),
    )?;
    let mut run = started["recording"].clone();
    let directory = run["directory"]
        .as_str()
        .context("start response has no recording directory")?
        .to_string();
    writeln_stderr(&format!("Local frames: {directory}"))?;
    let deadline =
        Instant::now() + Duration::from_millis(u64::from(duration_ms)) + Duration::from_secs(30);
    while !data::terminal(run["phase"].as_str().unwrap_or("")) {
        if CANCELLED.load(Ordering::Relaxed) {
            request(port, "stop", Some(&json!({"cancel":true})))?;
            bail!(
                "cancelled; saved frames remain local at {directory}; poll status before encoding"
            );
        }
        if Instant::now() >= deadline {
            request(port, "stop", Some(&json!({})))?;
            bail!("drain timed out; inspect status and the local ledger at {directory}");
        }
        std::thread::sleep(Duration::from_millis(200));
        run = request(port, "status", None)?["recording"].clone();
        ensure!(
            run["directory"].as_str() == Some(directory.as_str()),
            "recorder run changed while polling; original frames: {directory}"
        );
    }
    ensure!(
        run["phase"] != "failed",
        "recording failed: {}; frames: {directory}",
        run["error"]
    );
    render::encode(Path::new(&directory), preview, false)
}

/// A record command keeps its Ctrl-C handler during conversion too.
pub(super) fn check_interrupted() -> Result<()> {
    ensure!(
        !CANCELLED.load(Ordering::Relaxed),
        "conversion cancelled; raw frames remain local"
    );
    Ok(())
}

fn writeln_stderr(message: &str) -> Result<()> {
    use std::io::Write;
    writeln!(std::io::stderr().lock(), "{message}")?;
    Ok(())
}
