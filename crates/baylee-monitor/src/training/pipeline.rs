//! A `train_ops` pipeline directory: its status, its stages and the newest
//! progress line of the running stage.
//!
//! Every read is bounded and stays inside the configured directory: nothing
//! below it is followed through a symlink, JSON files are capped at 64 KiB and
//! parsed for a few fields only (a job's command line and environment are
//! never kept), and only the last 8 KiB of a log are read. Text that reaches
//! the browser has absolute paths reduced to their last component.

use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom},
    path::Path,
    time::UNIX_EPOCH,
};

const MAX_JSON: u64 = 64 * 1024;
const LOG_TAIL: u64 = 8 * 1024;
const MAX_STAGES: usize = 48;
const MAX_ENTRIES: usize = 256;
const MAX_TEXT: usize = 240;
const MAX_FIELDS: usize = 8;

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct Pipeline {
    /// The directory's last two path components.
    pub name: String,
    /// Why the status could not be read, when it could not.
    pub problem: Option<String>,
    pub phase: Option<String>,
    pub updated_ms: Option<u64>,
    pub promoted: Option<bool>,
    pub error: Option<String>,
    pub controller_pid: Option<u32>,
    /// Whether the recorded controller process still runs, judged by boot id
    /// and process start time; `None` when that cannot be told.
    pub controller_alive: Option<bool>,
    pub stages: Vec<Stage>,
    pub progress: Option<Progress>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Stage {
    pub name: String,
    pub status: Option<String>,
    pub started_ms: Option<u64>,
    pub finished_ms: Option<u64>,
    pub returncode: Option<i64>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Progress {
    pub stage: String,
    /// The newest `[tool] …` line of the stage's logs, paths reduced.
    pub line: String,
    /// `key=value` pairs of that line, such as `step` and `loss`.
    pub fields: Vec<(String, String)>,
    /// When that log was last written.
    pub updated_ms: Option<u64>,
}

#[derive(Deserialize)]
struct StatusFile {
    phase: Option<String>,
    updated_unix: Option<f64>,
    promoted: Option<bool>,
    error: Option<String>,
    controller: Option<Controller>,
}

#[derive(Deserialize)]
struct Controller {
    pid: Option<u32>,
    start_ticks: Option<u64>,
    boot_id: Option<String>,
}

#[derive(Deserialize)]
struct JobFile {
    status: Option<String>,
    started_at_unix: Option<f64>,
    finished_at_unix: Option<f64>,
    returncode: Option<i64>,
    reason: Option<String>,
}

/// Reads one pipeline directory; `boot_id` is this boot's id, to tell a live
/// controller from a recycled pid.
pub(crate) fn read(dir: &Path, boot_id: Option<&str>) -> Pipeline {
    let mut pipeline = Pipeline {
        name: display_name(dir),
        ..Pipeline::default()
    };
    if !fs::metadata(dir).is_ok_and(|m| m.is_dir()) {
        pipeline.problem = Some("Verzeichnis nicht gefunden".into());
        return pipeline;
    }
    match read_json::<StatusFile>(&dir.join("status.json")) {
        Ok(status) => {
            pipeline.phase = status.phase.map(|p| clean(&p));
            pipeline.updated_ms = status.updated_unix.and_then(unix_ms);
            pipeline.promoted = status.promoted;
            pipeline.error = status.error.map(|e| clean(&e));
            if let Some(controller) = status.controller {
                pipeline.controller_pid = controller.pid;
                pipeline.controller_alive = controller_alive(&controller, boot_id);
            }
        }
        Err(error) => pipeline.problem = Some(format!("status.json nicht lesbar ({error})")),
    }
    pipeline.stages = stages(&dir.join("jobs"));
    let current = pipeline
        .stages
        .iter()
        .rev()
        .find(|s| s.status.as_deref() == Some("running"))
        .or_else(|| {
            let phase = pipeline.phase.as_deref()?;
            pipeline.stages.iter().find(|s| s.name == phase)
        })
        .or_else(|| pipeline.stages.last());
    pipeline.progress =
        current.and_then(|stage| progress(&dir.join("jobs").join(&stage.name), &stage.name));
    pipeline
}

fn display_name(dir: &Path) -> String {
    let mut parts: Vec<_> = dir
        .components()
        .rev()
        .filter_map(|c| match c {
            std::path::Component::Normal(name) => Some(name.to_string_lossy()),
            _ => None,
        })
        .take(2)
        .collect();
    parts.reverse();
    parts.join("/")
}

/// Reads a small regular file (not a symlink) and parses the wanted fields.
fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> io::Result<T> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "keine Datei"));
    }
    if meta.len() > MAX_JSON {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "zu groß"));
    }
    let mut bytes = Vec::new();
    File::open(path)?.take(MAX_JSON).read_to_end(&mut bytes)?;
    serde_json::from_slice(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn unix_ms(seconds: f64) -> Option<u64> {
    (seconds.is_finite() && seconds > 0.0).then_some((seconds * 1000.0) as u64)
}

fn controller_alive(controller: &Controller, boot_id: Option<&str>) -> Option<bool> {
    let pid = controller.pid?;
    let start_ticks = controller.start_ticks?;
    match (controller.boot_id.as_deref(), boot_id) {
        (Some(recorded), Some(current)) if recorded != current => return Some(false),
        (_, None) => return None,
        _ => {}
    }
    let stat = match fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => stat,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Some(false),
        Err(_) => return None,
    };
    Some(process_start_ticks(&stat)? == start_ticks)
}

/// Field 22 of `/proc/<pid>/stat`, after the parenthesised command name.
fn process_start_ticks(stat: &str) -> Option<u64> {
    stat.rsplit_once(')')?
        .1
        .split_whitespace()
        .nth(19)?
        .parse()
        .ok()
}

fn stages(jobs: &Path) -> Vec<Stage> {
    if !fs::symlink_metadata(jobs).is_ok_and(|m| m.is_dir()) {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(jobs) else {
        return Vec::new();
    };
    let mut stages: Vec<Stage> = entries
        .take(MAX_ENTRIES)
        .flatten()
        // `file_type` does not follow symlinks.
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|entry| {
            let name = entry
                .file_name()
                .to_str()
                .filter(|n| valid_name(n))?
                .to_owned();
            let job = read_json::<JobFile>(&entry.path().join("job.json")).ok();
            Some(match job {
                Some(job) => Stage {
                    name,
                    status: job.status.map(|s| clean(&s)),
                    started_ms: job.started_at_unix.and_then(unix_ms),
                    finished_ms: job.finished_at_unix.and_then(unix_ms),
                    returncode: job.returncode,
                    reason: job.reason.map(|r| clean(&r)),
                },
                None => Stage {
                    name,
                    status: None,
                    started_ms: None,
                    finished_ms: None,
                    returncode: None,
                    reason: None,
                },
            })
        })
        .collect();
    stages.sort_by(|a, b| {
        (a.started_ms.is_none(), a.started_ms, &a.name).cmp(&(
            b.started_ms.is_none(),
            b.started_ms,
            &b.name,
        ))
    });
    let excess = stages.len().saturating_sub(MAX_STAGES);
    stages.drain(..excess);
    stages
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// The newest progress line in the stage's `stdout.log` or `stderr.log`,
/// whichever was written last.
fn progress(stage_dir: &Path, stage: &str) -> Option<Progress> {
    let (line, updated_ms) = ["stdout.log", "stderr.log"]
        .iter()
        .filter_map(|file| last_progress_line(&stage_dir.join(file)))
        .max_by_key(|(_, updated)| *updated)?;
    let line = clean(&line);
    Some(Progress {
        stage: stage.to_owned(),
        fields: fields(&line),
        line,
        updated_ms,
    })
}

fn last_progress_line(path: &Path) -> Option<(String, Option<u64>)> {
    let meta = fs::symlink_metadata(path).ok()?;
    if !meta.is_file() {
        return None;
    }
    let start = meta.len().saturating_sub(LOG_TAIL);
    let mut file = File::open(path).ok()?;
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut bytes = Vec::new();
    file.take(LOG_TAIL).read_to_end(&mut bytes).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    // A tail that starts mid-file starts mid-line.
    let text = if start > 0 {
        text.split_once('\n').map_or("", |(_, rest)| rest)
    } else {
        &text
    };
    let line = text.lines().rev().map(str::trim).find(|l| is_progress(l))?;
    let updated = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64);
    Some((line.to_owned(), updated))
}

/// A line our tools print as progress: `[tool] …`.
fn is_progress(line: &str) -> bool {
    line.strip_prefix('[')
        .and_then(|rest| rest.split_once("] "))
        .is_some_and(|(tag, _)| {
            (1..=32).contains(&tag.len())
                && tag
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
        })
}

fn fields(line: &str) -> Vec<(String, String)> {
    line.split_whitespace()
        .filter_map(|token| {
            let (key, value) = token.split_once('=')?;
            let key_ok = (1..=24).contains(&key.len())
                && key.starts_with(|c: char| c.is_ascii_lowercase())
                && key
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
            let value_ok = (1..=24).contains(&value.len()) && !value.contains(['/', '…']);
            (key_ok && value_ok).then(|| (key.to_owned(), value.to_owned()))
        })
        .take(MAX_FIELDS)
        .collect()
}

/// Display text: control characters removed, absolute paths reduced to
/// `…/name`, at most `MAX_TEXT` characters.
fn clean(text: &str) -> String {
    let text: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let redacted = text
        .split(' ')
        .map(redact_path)
        .collect::<Vec<_>>()
        .join(" ");
    let mut chars = redacted.chars();
    let mut clipped: String = chars.by_ref().take(MAX_TEXT).collect();
    if chars.next().is_some() {
        clipped.pop();
        clipped.push('…');
    }
    clipped
}

/// Replaces an absolute (or home-relative) path inside one token with
/// `…/last-component`, keeping what precedes it (`--out=`, `(`) and the
/// punctuation that follows it.
fn redact_path(token: &str) -> Cow<'_, str> {
    let starts_path = |at: usize| {
        let rest = &token[at..];
        let begins = rest.starts_with('/') || rest.starts_with("~/");
        let after_boundary = token[..at]
            .chars()
            .next_back()
            .is_none_or(|p| matches!(p, '=' | ':' | '(' | '"' | '\'' | '[' | ','));
        begins && after_boundary
    };
    let Some(at) = token
        .char_indices()
        .map(|(i, _)| i)
        .find(|&i| starts_path(i))
    else {
        return Cow::Borrowed(token);
    };
    let rest = &token[at..];
    let end = rest
        .find([')', ']', '"', '\'', ',', ';'])
        .unwrap_or(rest.len());
    let (path, tail) = rest.split_at(end);
    let (path, colon) = path.strip_suffix(':').map_or((path, ""), |p| (p, ":"));
    let name = path
        .rsplit('/')
        .find(|s| !s.is_empty() && *s != "~")
        .unwrap_or("");
    Cow::Owned(format!(
        "{}…/{name}{colon}{}",
        &token[..at],
        redact_path(tail)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    struct Scratch(std::path::PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("baylee-pulse-test-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn write(&self, relative: &str, text: &str) {
            let path = self.0.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::File::create(path)
                .unwrap()
                .write_all(text.as_bytes())
                .unwrap();
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const JOB_DONE: &str = r#"{"status":"complete","started_at_unix":100.5,"finished_at_unix":160.0,
        "returncode":0,"reason":null,"command":["/secret/bin","--token","hunter2"],
        "runtime_environment":{"API_KEY":"hunter2"}}"#;
    const JOB_RUNNING: &str = r#"{"status":"running","started_at_unix":161.0,"command":["x"]}"#;

    #[test]
    fn reads_phase_stages_and_the_running_stage_progress() {
        let scratch = Scratch::new("pipeline");
        let dir = scratch.0.join("v6x-exit").join("execution-x1");
        let at = |p: &str| format!("v6x-exit/execution-x1/{p}");
        scratch.write(
            &at("status.json"),
            r#"{"phase":"train-X1a","promoted":false,"updated_unix":1793695273.4072437,
                "manifest":"/home/ace/plan.json","controller":{"pid":1,"start_ticks":5,"boot_id":"other-boot"}}"#,
        );
        scratch.write(&at("jobs/convert-x1/job.json"), JOB_DONE);
        scratch.write(
            &at("jobs/convert-x1/stderr.log"),
            "[convert4] converted 2043 games\n",
        );
        scratch.write(&at("jobs/train-X1a/job.json"), JOB_RUNNING);
        scratch.write(
            &at("jobs/train-X1a/stdout.log"),
            "[train4] loaded initial weights from /home/ace/baylee-data/net.pt\n\
             [train4] step=2150 epoch=1 loss=0.9757 batches_per_s=1.48\n\
             {\"val_loss\": 0.5}\n",
        );
        scratch.write(&at("jobs/train-X1a/stderr.log"), "UserWarning: something\n");
        let pipeline = read(&dir, Some("this-boot"));
        assert_eq!(pipeline.name, "v6x-exit/execution-x1");
        assert_eq!(pipeline.problem, None);
        assert_eq!(pipeline.phase.as_deref(), Some("train-X1a"));
        assert_eq!(pipeline.updated_ms, Some(1_793_695_273_407));
        assert_eq!(pipeline.controller_pid, Some(1));
        assert_eq!(
            pipeline.controller_alive,
            Some(false),
            "another boot's controller"
        );
        let names: Vec<_> = pipeline.stages.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["convert-x1", "train-X1a"]);
        assert_eq!(pipeline.stages[0].returncode, Some(0));
        assert_eq!(pipeline.stages[0].finished_ms, Some(160_000));
        let progress = pipeline.progress.unwrap();
        assert_eq!(progress.stage, "train-X1a");
        assert_eq!(
            progress.line,
            "[train4] step=2150 epoch=1 loss=0.9757 batches_per_s=1.48"
        );
        assert_eq!(progress.fields.len(), 4);
        assert_eq!(progress.fields[2], ("loss".into(), "0.9757".into()));
        let json = serde_json::to_string(&read(&dir, None)).unwrap();
        assert!(
            !json.contains("hunter2") && !json.contains("/home/ace"),
            "{json}"
        );
    }

    #[test]
    fn a_missing_or_broken_pipeline_says_why() {
        let scratch = Scratch::new("broken");
        let missing = read(&scratch.0.join("nope"), None);
        assert_eq!(
            missing.problem.as_deref(),
            Some("Verzeichnis nicht gefunden")
        );
        scratch.write("status.json", "{not json");
        let broken = read(&scratch.0, None);
        assert!(
            broken
                .problem
                .unwrap()
                .starts_with("status.json nicht lesbar")
        );
        assert!(broken.stages.is_empty());
    }

    #[test]
    fn symlinks_below_the_pipeline_are_not_followed() {
        let scratch = Scratch::new("links");
        scratch.write("outside/job.json", JOB_DONE);
        scratch.write("outside/stdout.log", "[leak] outside\n");
        scratch.write("pipe/status.json", r#"{"phase":"x"}"#);
        fs::create_dir_all(scratch.0.join("pipe/jobs/real")).unwrap();
        std::os::unix::fs::symlink(
            scratch.0.join("outside"),
            scratch.0.join("pipe/jobs/linked"),
        )
        .unwrap();
        std::os::unix::fs::symlink(
            scratch.0.join("outside/stdout.log"),
            scratch.0.join("pipe/jobs/real/stdout.log"),
        )
        .unwrap();
        let pipeline = read(&scratch.0.join("pipe"), None);
        let names: Vec<_> = pipeline.stages.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["real"]);
        assert_eq!(pipeline.progress, None);
    }

    #[test]
    fn reads_only_the_tail_of_a_long_log() {
        let scratch = Scratch::new("tail");
        let mut log = String::from("[train4] step=1 an early line\n");
        log.push_str(&"x".repeat(3 * LOG_TAIL as usize));
        log.push_str("\n[train4] step=99 loss=0.5\nno tag here\n");
        scratch.write("stdout.log", &log);
        let (line, _) = last_progress_line(&scratch.0.join("stdout.log")).unwrap();
        assert_eq!(line, "[train4] step=99 loss=0.5");
        scratch.write(
            "only-old.log",
            &format!("[t] old\n{}\n", "y".repeat(2 * LOG_TAIL as usize)),
        );
        assert_eq!(last_progress_line(&scratch.0.join("only-old.log")), None);
    }

    #[test]
    fn paths_are_reduced_and_text_is_bounded() {
        assert_eq!(
            clean("already owned: /home/ace/baylee-data/v5/gpu-work.lock: lock failed"),
            "already owned: …/gpu-work.lock: lock failed"
        );
        assert_eq!(
            clean("--out=/a/b/c.json (/x/y) 1/2"),
            "--out=…/c.json (…/y) 1/2"
        );
        assert_eq!(clean("runs /a/x1,/a/x2 ~/k"), "runs …/x1,…/x2 …/k");
        assert_eq!(clean("tab\there\x1b[0m"), "tab here [0m");
        let long = clean(&"é".repeat(500));
        assert_eq!(long.chars().count(), MAX_TEXT);
        assert!(long.ends_with('…'));
        assert!(is_progress("[train4] step=1"));
        assert!(!is_progress("[] x") && !is_progress("[a b] x") && !is_progress("plain"));
        assert_eq!(
            process_start_ticks("1 (a) b) S 1 1 1 0 -1 0 0 0 0 0 0 0 0 0 20 0 1 0 5752229 0"),
            Some(5_752_229)
        );
    }
}
