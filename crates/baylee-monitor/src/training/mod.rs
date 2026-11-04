//! The training panel's data: the `baylee-*` system units and the configured
//! `train_ops` pipelines, refreshed every few seconds on their own thread so a
//! slow bus or disk never delays the 1 Hz sampler. Read-only, like the rest.

mod dbus;
mod pipeline;
mod units;

use serde::Serialize;
use std::{
    path::{Component, PathBuf},
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub(crate) use pipeline::Pipeline;
pub(crate) use units::{Services, Unit};

/// Training changes over minutes; five seconds keeps the panel current.
const REFRESH: Duration = Duration::from_secs(5);
/// More would not fit on a phone and could make a refresh slow.
pub(crate) const MAX_PIPELINES: usize = 8;

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct Training {
    /// When this view was assembled; zero before the first refresh.
    pub checked_ms: u64,
    pub units: Vec<Unit>,
    /// Why systemd could not be asked, when it could not.
    pub units_problem: Option<String>,
    /// The machine's other services, for a failed one to stand out.
    pub services: Option<Services>,
    pub pipelines: Vec<Pipeline>,
}

/// Parses one `--pipeline` value: an absolute directory without `..`.
pub(crate) fn parse_pipeline_dir(value: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err("a pipeline directory must be an absolute path".into());
    }
    if path.components().any(|c| c == Component::ParentDir) {
        return Err("a pipeline directory must not contain '..'".into());
    }
    Ok(path)
}

/// Starts the refresh thread, which replaces `shared` after every round.
pub(crate) fn spawn(pipelines: Vec<PathBuf>, shared: Arc<Mutex<Training>>) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name("training".into())
        .spawn(move || {
            let mut units = units::Units::default();
            loop {
                let started = Instant::now();
                let view = collect(&mut units, &pipelines);
                *shared.lock().unwrap_or_else(PoisonError::into_inner) = view;
                std::thread::sleep(REFRESH.saturating_sub(started.elapsed()));
            }
        })?;
    Ok(())
}

fn collect(units: &mut units::Units, pipelines: &[PathBuf]) -> Training {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let units_problem = units
        .refresh(now_ms)
        .err()
        .map(|error| format!("systemd nicht erreichbar: {error}"));
    let boot_id = std::fs::read_to_string("/proc/sys/kernel/random/boot_id").ok();
    let boot_id = boot_id.as_deref().map(str::trim);
    Training {
        checked_ms: now_ms,
        units: units.view(),
        services: units.services(),
        units_problem,
        pipelines: pipelines
            .iter()
            .map(|dir| pipeline::read(dir, boot_id))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_directories_are_absolute_and_plain() {
        assert_eq!(
            parse_pipeline_dir("/home/ace/baylee-data/v6/train/v6x-exit/execution-x1"),
            Ok(PathBuf::from(
                "/home/ace/baylee-data/v6/train/v6x-exit/execution-x1"
            ))
        );
        assert!(parse_pipeline_dir("baylee-data/x").is_err());
        assert!(parse_pipeline_dir("/home/ace/../etc").is_err());
    }
}
