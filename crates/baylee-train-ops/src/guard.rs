//! Bounded jobs with isolated process groups and durable, immutable attempts.
use crate::{files, process};
use anyhow::{Context, Result, bail, ensure};
use rustix::process::{Pid, Signal, WaitId, WaitIdOptions, kill_process_group, waitid};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// One bounded command in an immutable attempt directory.
pub struct Job {
    /// Attempt namespace; failed attempts are never silently reused.
    pub directory: PathBuf,
    /// Executable and literal arguments; no shell interpolation.
    pub command: Vec<String>,
    /// Source checkout and command working directory.
    pub cwd: PathBuf,
    /// Maximum elapsed wall clock time, including startup.
    pub timeout_seconds: f64,
    /// Process group proportional memory limit in GiB.
    pub memory_gib: f64,
    /// Immutable input files hashed before and after the command.
    #[serde(default)]
    pub inputs: Vec<PathBuf>,
}

/// The unreaped direct child pins its PID and process group during cleanup.
struct OwnedGroup {
    child: Child,
    pid: Pid,
    reaped: bool,
}
impl OwnedGroup {
    fn exited(&self) -> Result<bool> {
        ensure!(!self.reaped, "child already reaped; ownership lost");
        Ok(waitid(
            WaitId::Pid(self.pid),
            WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
        )?
        .is_some())
    }
    fn signal(&self, signal: Signal) -> Result<()> {
        self.exited()?;
        match kill_process_group(self.pid, signal) {
            Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
    fn cleanup(&mut self) -> Result<ExitStatus> {
        self.exited()?;
        if !process::group_members(self.child.id())?.is_empty() {
            self.signal(Signal::TERM)?;
            let deadline = Instant::now() + Duration::from_secs(5);
            while !process::group_members(self.child.id())?.is_empty() && Instant::now() < deadline
            {
                thread::sleep(Duration::from_millis(20));
            }
            if !process::group_members(self.child.id())?.is_empty() {
                self.signal(Signal::KILL)?;
            }
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while !self.exited()? || !process::group_members(self.child.id())?.is_empty() {
            ensure!(
                Instant::now() < deadline,
                "owned descendants remain; do not reap/reuse leader identity"
            );
            thread::sleep(Duration::from_millis(20));
        }
        let status = self.child.wait()?;
        self.reaped = true;
        Ok(status)
    }
}
impl Drop for OwnedGroup {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.cleanup();
        }
    }
}

#[cfg(test)]
mod tests;

/// Capture a bounded control command without leaking descendants on timeout.
pub(crate) fn capture(program: &Path, arguments: &[&str], timeout: Duration) -> Result<String> {
    let mut output = tempfile::tempfile()?;
    let child = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(output.try_clone()?)
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()?;
    let pid = Pid::from_raw(i32::try_from(child.id())?).context("invalid child PID")?;
    let mut group = OwnedGroup {
        child,
        pid,
        reaped: false,
    };
    let started = Instant::now();
    while !group.exited()? && started.elapsed() < timeout {
        thread::sleep(Duration::from_millis(20));
    }
    let timed_out = !group.exited()?;
    let status = group.cleanup()?;
    ensure!(
        !timed_out && status.success(),
        "bounded control command failed or exceeded deadline"
    );
    ensure!(
        output.metadata()?.len() <= 64 * 1024,
        "oversized control-command receipt"
    );
    output.seek(SeekFrom::Start(0))?;
    let mut text = String::new();
    output.read_to_string(&mut text)?;
    Ok(text)
}

#[derive(Debug, PartialEq, Eq)]
struct Executable {
    // Do not canonicalize the invocation: runtimes discover environments from argv[0].
    invocation: PathBuf,
    canonical: PathBuf,
}

fn executable(command: &str, cwd: &Path) -> Result<Executable> {
    executable_in_path(command, cwd, &std::env::var_os("PATH").unwrap_or_default())
}

fn executable_in_path(
    command: &str,
    cwd: &Path,
    search_path: &std::ffi::OsStr,
) -> Result<Executable> {
    let cwd = std::path::absolute(cwd)?;
    let resolve = |invocation: PathBuf| -> Result<Executable> {
        let canonical = invocation.canonicalize()?;
        ensure!(canonical.is_file(), "executable is not a file: {command}");
        Ok(Executable {
            invocation,
            canonical,
        })
    };
    let path = Path::new(command);
    if command.contains(std::path::MAIN_SEPARATOR) || path.is_absolute() {
        return resolve(cwd.join(path));
    }
    for directory in std::env::split_paths(search_path) {
        let candidate = cwd.join(directory).join(path);
        if candidate.is_file() {
            return resolve(candidate);
        }
    }
    bail!("executable not found: {command}")
}

fn input_hashes(job: &Job) -> Result<BTreeMap<String, String>> {
    job.inputs
        .iter()
        .map(|p| {
            Ok((
                p.to_string_lossy().into_owned(),
                files::sha256(&job.cwd.join(p))?,
            ))
        })
        .collect()
}

/// Validated attempt inputs and durable starting state.
struct Prepared {
    exe: Executable,
    exe_hash: String,
    inputs: BTreeMap<String, String>,
    state: serde_json::Value,
}

fn source_identity(cwd: &Path) -> Result<(String, Vec<u8>)> {
    let revision = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(cwd)
        .output()?;
    ensure!(
        revision.status.success(),
        "job cwd must have a source identity"
    );
    let diff = Command::new("git")
        .args(["diff", "--binary", "HEAD"])
        .current_dir(cwd)
        .output()?;
    ensure!(diff.status.success(), "cannot snapshot tracked sources");
    Ok((
        String::from_utf8(revision.stdout)?.trim().to_owned(),
        diff.stdout,
    ))
}

fn runtime_environment() -> BTreeMap<&'static str, String> {
    [
        "BAYLEE_EP",
        "BAYLEE_TRAIN_OPS",
        "CUDA_VISIBLE_DEVICES",
        "OMP_NUM_THREADS",
        "MKL_NUM_THREADS",
        "LD_LIBRARY_PATH",
        "CUBLAS_WORKSPACE_CONFIG",
        "PYTHONPATH",
        "PYTHONHASHSEED",
    ]
    .into_iter()
    .filter_map(|key| std::env::var(key).ok().map(|value| (key, value)))
    .collect()
}

fn spawn(job: &Job, exe: &Path) -> Result<OwnedGroup> {
    let child = Command::new(exe)
        .args(&job.command[1..])
        .current_dir(&job.cwd)
        .stdin(Stdio::null())
        .stdout(File::create(job.directory.join("stdout.log"))?)
        .stderr(File::create(job.directory.join("stderr.log"))?)
        .process_group(0)
        .spawn()?;
    let pid = Pid::from_raw(i32::try_from(child.id())?).context("invalid child PID")?;
    Ok(OwnedGroup {
        child,
        pid,
        reaped: false,
    })
}

fn prepare(job: &Job) -> Result<Option<Prepared>> {
    ensure!(!job.command.is_empty(), "empty job command");
    ensure!(
        job.timeout_seconds.is_finite() && job.timeout_seconds > 0.0,
        "finite positive timeout required"
    );
    ensure!(
        job.memory_gib.is_finite() && job.memory_gib > 0.0,
        "finite positive memory bound required"
    );
    let manifest = job.directory.join("job.json");
    let exe = executable(&job.command[0], &job.cwd)?;
    let exe_hash = files::sha256(&exe.canonical)?;
    let inputs = input_hashes(job)?;
    let (revision, diff) = source_identity(&job.cwd)?;
    let diff_sha = files::hex(&Sha256::digest(&diff));
    let environment = runtime_environment();
    if manifest.exists() {
        let old = files::read_json(&manifest)?;
        ensure!(
            old["status"] == "complete"
                && old["command"] == json!(job.command)
                && old["cwd"] == json!(job.cwd)
                && old["executable_invocation"] == json!(exe.invocation)
                && old["executable"] == json!(exe.canonical)
                && old["executable_sha256"] == exe_hash
                && old["inputs"] == json!(inputs)
                && old["revision"] == revision
                && old["tracked_diff_sha256"] == diff_sha
                && old["runtime_environment"] == json!(environment)
                && old["timeout_seconds"] == json!(job.timeout_seconds)
                && old["memory_limit_gib"] == json!(job.memory_gib),
            "preserving existing attempt {}; use a new directory",
            manifest.display()
        );
        return Ok(None);
    }
    fs::write(job.directory.join("source.patch"), &diff)?;
    let state = json!({"schema_version":3,"guard":"rust-owned-group-v1","command":job.command,"cwd":job.cwd,"revision":revision,"runtime_environment":environment,"executable_invocation":exe.invocation,"executable":exe.canonical,"executable_sha256":exe_hash,"tracked_diff_sha256":diff_sha,"inputs":inputs,"status":"starting","started_at_unix":files::unix_time()?,"timeout_seconds":job.timeout_seconds,"memory_limit_gib":job.memory_gib});
    files::atomic_json(&manifest, &state)?;
    Ok(Some(Prepared {
        exe,
        exe_hash,
        inputs,
        state,
    }))
}

/// Execute a job, record its result and clean up its entire owned process group.
/// # Errors
/// Rejects invalid/reused attempts, accounting failures and incomplete cleanup.
/// A normal unsuccessful command returns `Ok(false)` with retained artifacts.
pub fn run(job: &Job, interrupted: &AtomicBool) -> Result<bool> {
    fs::create_dir_all(&job.directory)?;
    let _lock = files::lock(&job.directory.join("guard.lock"))?;
    let Some(Prepared {
        exe,
        exe_hash,
        inputs,
        mut state,
    }) = prepare(job)?
    else {
        return Ok(true);
    };
    let manifest = job.directory.join("job.json");
    let started = Instant::now();
    let mut group: Option<OwnedGroup> = None;
    let mut peak = 0;
    let mut reason: Option<String> = None;
    let attempt = (|| -> Result<()> {
        ensure!(
            !interrupted.load(Ordering::Relaxed),
            "interrupted before spawn"
        );
        ensure!(
            executable(&job.command[0], &job.cwd)? == exe
                && files::sha256(&exe.canonical)? == exe_hash,
            "executable binding changed before spawn"
        );
        group = Some(spawn(job, &exe.invocation)?);
        let child = group.as_ref().context("owned child")?;
        state["pid"] = json!(child.child.id());
        state["child_identity"] = json!(process::identity(child.child.id()));
        state["controller"] = json!(process::identity(std::process::id()));
        state["status"] = json!("running");
        files::atomic_json(&manifest, &state)?;
        while !child.exited()? {
            if interrupted.load(Ordering::Relaxed) {
                reason = Some("interrupted".into());
                break;
            }
            let pss = process::group_pss_bytes(child.child.id())?;
            peak = peak.max(pss);
            if started.elapsed().as_secs_f64() >= job.timeout_seconds {
                reason = Some("deadline".into());
                break;
            }
            #[allow(
                clippy::cast_precision_loss,
                reason = "memory limit comparison, not model or engine arithmetic"
            )]
            if pss as f64 > job.memory_gib * 1_073_741_824.0 {
                reason = Some("memory_limit".into());
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        Ok(())
    })();
    if let Err(ref error) = attempt {
        reason = Some(format!("exception: {error:#}"));
    }
    let cleanup = group.as_mut().map(OwnedGroup::cleanup).transpose();
    // `cleanup` succeeds only after the owned leader and every live group member
    // have exited, before the leader is reaped. No spawned child is not a proof.
    state["cleanup_verified"] = json!(cleanup.as_ref().is_ok_and(Option::is_some));
    if let Err(ref error) = cleanup {
        reason = Some(format!("cleanup_failed: {error:#}"));
    }
    let validation = input_hashes(job).and_then(|current| {
        Ok(current == inputs
            && executable(&job.command[0], &job.cwd)? == exe
            && files::sha256(&exe.canonical)? == exe_hash)
    });
    if reason.is_none() {
        match validation {
            Ok(true) => {}
            Ok(false) => reason = Some("input_changed".into()),
            Err(error) => reason = Some(format!("input_validation_failed: {error:#}")),
        }
    }
    let success = cleanup
        .as_ref()
        .ok()
        .and_then(|v| v.as_ref())
        .is_some_and(ExitStatus::success)
        && reason.is_none();
    state["status"] = json!(if success { "complete" } else { "quarantined" });
    state["reason"] = json!(reason);
    state["returncode"] = json!(
        cleanup
            .as_ref()
            .ok()
            .and_then(|v| v.as_ref())
            .and_then(ExitStatus::code)
    );
    state["elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
    state["peak_pss_bytes"] = json!(peak);
    state["finished_at_unix"] = json!(files::unix_time()?);
    files::atomic_json(&manifest, &state)?;
    cleanup?;
    attempt?;
    Ok(success)
}
