//! Local preview selection and bounded official gateway export collection.
use super::{atomic_json, live_import::import_records, publication::publish, read_json};
use crate::files::unix_time;
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

/// Explicit inputs to local playtest maintenance.
pub struct Settings {
    /// Frozen experiment directory.
    pub experiment: PathBuf,
    /// Preview selection output.
    pub selection: PathBuf,
    /// Official gateway executable, used only for anonymized export.
    pub gateway_bin: PathBuf,
    /// Train-only content-addressed record store.
    pub corpus: PathBuf,
    /// Neural seat in a two-seat game.
    pub neural_seat: u64,
    /// Maintenance status output.
    pub status: PathBuf,
}
struct ExportDirectory(PathBuf);
impl ExportDirectory {
    fn new() -> Result<Self> {
        let path =
            std::env::temp_dir().join(format!("baylee-live-export-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir(&path)?;
        Ok(Self(path))
    }
}
impl Drop for ExportDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
struct ExportChild(Child);
impl Drop for ExportChild {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}
fn export(command: &mut Command, stop: &AtomicBool, budget: Duration) -> Result<()> {
    ensure!(!stop.load(Ordering::Relaxed), "maintenance stopped");
    let mut child = ExportChild(command.stdin(Stdio::null()).stdout(Stdio::null()).spawn()?);
    let started = Instant::now();
    loop {
        if let Some(status) = child.0.try_wait()? {
            ensure!(status.success(), "official gateway export failed");
            return Ok(());
        }
        if stop.load(Ordering::Relaxed) || started.elapsed() >= budget {
            child.0.kill()?;
            child.0.wait()?;
            anyhow::bail!("official gateway export stopped or exceeded its deadline");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
/// Run the official anonymized exporter, then stage its records without policy targets.
///
/// # Errors
/// Rejects failed or interrupted exports and invalid anonymized record batches.
pub fn collect(settings: &Settings, stop: &AtomicBool) -> Result<Value> {
    let directory = ExportDirectory::new()?;
    export(
        Command::new(&settings.gateway_bin)
            .args(["records", "export", "--out"])
            .arg(&directory.0),
        stop,
        Duration::from_secs(120),
    )?;
    import_records(&directory.0, &settings.corpus, settings.neural_seat, None)
}

/// Return the adjacent confirmation review status path.
pub fn confirmation_status_path(output: &Path) -> PathBuf {
    let name = output.file_name().unwrap_or_default().to_string_lossy();
    output.with_file_name(format!("{name}.confirmation-status.json"))
}

/// Run periodic preview selection and collection until interrupted, or one bounded tick.
///
/// # Errors
/// Requires exporter credentials in the environment and a writable status path.
pub fn run(settings: &Settings, stop: &AtomicBool, once: bool) -> Result<()> {
    ensure!(
        std::env::var_os("DATABASE_URL").is_some_and(|v| !v.is_empty()),
        "Official export needs DATABASE_URL in the service environment"
    );
    ensure!(settings.neural_seat < 2, "invalid neural seat");
    let mut state = json!({"phase":"watching","pid":std::process::id(),"errors":{},"selection":settings.selection,"corpus":settings.corpus});
    let mut next_selection = Instant::now();
    let mut next_collection = next_selection;
    while !stop.load(Ordering::Relaxed) {
        let now = Instant::now();
        if now >= next_selection {
            let result = (|| -> Result<()> {
                if publish(&settings.experiment, &settings.selection)? {
                    state["selection_updated_unix"] = json!(unix_time()?);
                }
                let path = confirmation_status_path(&settings.selection);
                state["selection_review"] = if path.exists() {
                    read_json(&path)?
                } else {
                    Value::Null
                };
                Ok(())
            })();
            record_error(&mut state, "selection", result);
            next_selection = now + Duration::from_secs(30);
        }
        if now >= next_collection && !stop.load(Ordering::Relaxed) {
            let result = (|| -> Result<()> {
                state["collection"] = collect(settings, stop)?;
                state["collection_checked_unix"] = json!(unix_time()?);
                Ok(())
            })();
            record_error(&mut state, "collection", result);
            next_collection = now + Duration::from_secs(300);
        }
        state["checked_unix"] = json!(unix_time()?);
        atomic_json(&settings.status, &state)?;
        if once {
            state["phase"] = json!("one_shot");
            atomic_json(&settings.status, &state)?;
            return Ok(());
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    state["phase"] = json!("stopped");
    atomic_json(&settings.status, &state)?;
    Ok(())
}
fn record_error(state: &mut Value, key: &str, result: Result<()>) {
    if let Err(error) = result {
        state["errors"][key] = json!(error.to_string());
    } else if let Some(errors) = state["errors"].as_object_mut() {
        errors.remove(key);
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn exporter_failure_is_reported_and_deadline_reaps_our_child() {
        let stop = AtomicBool::new(false);
        assert!(export(&mut Command::new("false"), &stop, Duration::from_secs(1)).is_err());
        let started = Instant::now();
        assert!(
            export(
                Command::new("sleep").arg("10"),
                &stop,
                Duration::from_millis(20)
            )
            .is_err()
        );
        assert!(started.elapsed() < Duration::from_secs(2));
        stop.store(true, Ordering::Relaxed);
        assert!(export(&mut Command::new("true"), &stop, Duration::from_secs(1)).is_err());
    }
}
