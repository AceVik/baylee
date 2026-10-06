//! Legacy rename-journal recovery and shared staging primitives.
//!
//! New native installs use [`crate::launch`], which keeps a permanent launch
//! path. These legacy renames require an independently runnable recovery
//! caller; they do not themselves keep the normal executable path present.
//!
//! The steps are [`plan`](crate::plan::plan)'s renames. Before the first
//! one the whole list is written to the installation state directory as `journal.json`,
//! and after each one the count done is written again; every write goes to
//! a temporary file that is renamed over the journal, so the journal is
//! always one whole version or the other.
//!
//! A crash can land between a rename and the count that records it, so at
//! the next start ([`recover`]) the one step after the recorded count is
//! checked on the disk: if its source is gone and its target is there, it
//! happened. From there the update is **finished** when every remaining
//! step can still be done (its source there, its target free), and **rolled
//! back** otherwise. Rollback direction and progress are durable too; an
//! error leaves the journal and payload for a later retry, rather than
//! claiming that restoration succeeded.
//!
//! The files beside the journal:
//!
//! - `staged.json`: an update is unpacked in `new/`, verified, ready.
//! - `applied.json`: an update finished; the next start says so once
//!   ([`take_applied`]) and deletes what was set aside.
//! - `failed.json`: an update could not be installed; the notice links to
//!   the release instead, and that version is not downloaded again.

use crate::plan::{self, NEW, Os, PlanError, Rel, Rename, STAGE};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

/// An OS-backed claim. The lock file is permanent: unlinking it would let
/// another process lock a different inode while the first still owns it.
pub(crate) struct Claim {
    _file: fs::File,
}

impl Claim {
    pub(crate) fn wait(stage: &Path, name: &str) -> io::Result<Self> {
        fs::create_dir_all(stage)?;
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(stage.join(name))?;
        file.lock()?;
        Ok(Self { _file: file })
    }
    pub(crate) fn take(stage: &Path) -> io::Result<Self> {
        Self::named(stage, "mutation.lock")
    }

    pub(crate) fn named(stage: &Path, name: &str) -> io::Result<Self> {
        fs::create_dir_all(stage)?;
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(stage.join(name))?;
        file.try_lock().map_err(io::Error::from)?;
        Ok(Self { _file: file })
    }
}

const JOURNAL: &str = "journal.json";
const STAGED: &str = "staged.json";
const APPLIED: &str = "applied.json";
const FAILED: &str = "failed.json";

/// Where the running client is installed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Install {
    /// Launcher-managed per-user state; legacy callers stage beside the app.
    #[serde(default)]
    pub state: Option<PathBuf>,
    /// The system, which picks the plan.
    pub os: Os,
    /// The folder the archive was unpacked into ([`crate::plan`]).
    pub base: PathBuf,
    /// What the program is called in it: `baylee-client(.exe)`, or the
    /// bundle's name on macOS.
    pub program: String,
}

/// Why the running client cannot be updated where it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unplaceable {
    /// The original installation is read-only; launch normally, link updates.
    ReadOnly,
    /// Started without the permanent launcher; no installation lease.
    NotLaunched,
    /// On macOS, a program that is not inside a `.app` bundle (a
    /// development build started by `cargo run`).
    NotABundle,
    /// macOS runs a quarantined app it was never allowed to move from a
    /// read-only copy (App Translocation), and would not say of what
    /// (`launch::Blocked::Translocated`; with an answer, the launcher keys
    /// the state by the original and installs). The client offers to move
    /// it to Applications (`crate::relocate`).
    Translocated,
    /// A program path with no folder around it.
    NoFolder,
}

impl std::fmt::Display for Unplaceable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::ReadOnly => "the installation is read-only",
            Self::NotLaunched => "start the client through its packaged launcher to update",
            Self::NotABundle => "the program is not inside an app bundle",
            Self::Translocated => "macOS runs this app from a read-only copy (App Translocation)",
            Self::NoFolder => "the program has no folder",
        })
    }
}

impl std::error::Error for Unplaceable {}

impl Install {
    /// The installation around the running program at `exe`
    /// (`std::env::current_exe()`), read at start: the name it answers to
    /// changes half way through an update on Windows.
    ///
    /// # Errors
    ///
    /// [`Unplaceable`] when there is no installation to replace.
    pub fn around(exe: &Path, os: Os) -> Result<Self, Unplaceable> {
        if os == Os::MacOs && exe.to_string_lossy().contains("/AppTranslocation/") {
            return Err(Unplaceable::Translocated);
        }
        Self::around_launcher(exe, os)
    }

    /// Locate the original package even under macOS App Translocation.
    /// The launcher may start a read-only package; its session separately
    /// disables installation. The updater's [`Self::around`] stays strict.
    ///
    /// # Errors
    /// A malformed package layout or missing parent directory.
    pub fn around_launcher(exe: &Path, os: Os) -> Result<Self, Unplaceable> {
        let name = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned());
        match os {
            Os::MacOs => {
                let macos = exe.parent().ok_or(Unplaceable::NoFolder)?;
                let contents = macos.parent().ok_or(Unplaceable::NotABundle)?;
                let bundle = contents.parent().ok_or(Unplaceable::NotABundle)?;
                let is_bundle = name(macos).as_deref() == Some("MacOS")
                    && name(contents).as_deref() == Some("Contents")
                    && bundle
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("app"));
                if !is_bundle {
                    return Err(Unplaceable::NotABundle);
                }
                Ok(Self {
                    state: None,
                    os,
                    base: bundle.parent().ok_or(Unplaceable::NoFolder)?.to_path_buf(),
                    program: name(bundle).ok_or(Unplaceable::NotABundle)?,
                })
            }
            Os::Windows | Os::Linux => Ok(Self {
                state: None,
                os,
                base: exe.parent().ok_or(Unplaceable::NoFolder)?.to_path_buf(),
                program: name(exe).ok_or(Unplaceable::NoFolder)?,
            }),
        }
    }

    /// Per-user launcher state, or `<base>/.baylee-update/<program>` for legacy callers.
    #[must_use]
    pub fn stage(&self) -> PathBuf {
        self.state
            .clone()
            .unwrap_or_else(|| self.base.join(STAGE).join(&self.program))
    }

    fn at(&self, rel: &[String]) -> PathBuf {
        if rel.first().is_some_and(|part| part == STAGE) {
            return rel[1..]
                .iter()
                .fold(self.stage(), |path, part| path.join(part));
        }
        rel.iter()
            .fold(self.base.clone(), |path, part| path.join(part))
    }

    /// Whether the installation can be replaced by this user: the staging
    /// directory can be made and written in the base. A folder the player
    /// cannot write (`Program Files`, a root-owned `/opt`) answers no, and
    /// the notice then links to the release instead of asking to elevate.
    ///
    /// # Errors
    ///
    /// The I/O error that said no.
    pub fn writable(&self) -> io::Result<()> {
        let stage = self.stage();
        fs::create_dir_all(&stage)?;
        let probe = stage.join(".probe");
        fs::write(&probe, b"")?;
        fs::remove_file(&probe)
    }
}

/// An update unpacked, verified and ready to install.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Staged {
    /// The version it installs.
    pub version: String,
    /// Its release tag.
    pub tag: String,
    /// The release page.
    pub page: String,
    /// The archive it came from, by name and size: a later check that finds
    /// the same asset does not download it again.
    pub asset: String,
    /// The archive's size in bytes.
    pub size: u64,
}

/// An update that finished.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Applied {
    /// The version it replaced.
    pub from: String,
    /// The version it installed.
    pub to: String,
    /// What the old installation was set aside as, to delete.
    pub asides: Vec<Rel>,
}

/// An update that could not be installed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Failed {
    /// The version it would have installed.
    pub version: String,
    /// Why, for the log and the notice.
    pub reason: String,
}

/// The renames of one update and how many are done.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Journal {
    /// The version being replaced.
    pub from: String,
    /// The version being installed.
    pub to: String,
    /// Every rename, in order.
    pub steps: Vec<Rename>,
    /// How many of them are known to be done.
    pub done: usize,
    /// Rollback is itself a durable transaction, resumed after a crash.
    #[serde(default)]
    pub rolling_back: bool,
}

/// Why an update was not installed.
#[derive(Debug)]
pub enum ApplyError {
    /// Nothing is staged.
    NothingStaged,
    /// An update is being downloaded or unpacked right now.
    Busy,
    /// The staged tree does not fit this installation.
    Plan(PlanError),
    /// A rename or a journal write failed; every done step was undone.
    Io(io::Error),
}

impl std::fmt::Display for ApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NothingStaged => f.write_str("no update is staged"),
            Self::Busy => f.write_str("an update is still being staged"),
            Self::Plan(err) => write!(f, "{err}"),
            Self::Io(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for ApplyError {}

impl From<io::Error> for ApplyError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

/// Writes `value` as JSON at `path` whole or not at all: a temporary file,
/// flushed to disk, renamed over the old one.
pub(crate) fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    let mut file = fs::File::create(&tmp)?;
    file.write_all(&serde_json::to_vec_pretty(value).map_err(io::Error::other)?)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&tmp, path)?;
    sync_dir(path.parent().unwrap_or(Path::new(".")));
    Ok(())
}

pub(crate) fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

/// Makes a rename in `dir` durable. Best effort: a directory cannot be
/// opened for this on Windows, whose renames are durable when they return.
pub(crate) fn sync_dir(dir: &Path) {
    #[cfg(unix)]
    if let Ok(handle) = fs::File::open(dir) {
        let _ = handle.sync_all();
    }
    #[cfg(not(unix))]
    let _ = dir;
}

/// Whether something is at `path`, a dangling link included.
fn there(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn remove_any(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

/// What is staged, if anything.
#[must_use]
pub fn staged(install: &Install) -> Option<Staged> {
    let staged: Staged = read_json(&install.stage().join(STAGED))?;
    install.stage().join(NEW).is_dir().then_some(staged)
}

/// Records a staged update. [`crate::check`] calls it once `new/` holds a
/// verified tree.
///
/// # Errors
///
/// The write's.
pub fn mark_staged(install: &Install, staged: &Staged) -> io::Result<()> {
    write_json(&install.stage().join(STAGED), staged)
}

/// The update that could not be installed, if one is recorded.
#[must_use]
pub fn failed(install: &Install) -> Option<Failed> {
    read_json(&install.stage().join(FAILED))
}

fn mark_failed(install: &Install, version: &str, reason: &str) {
    let _ = fs::create_dir_all(install.stage());
    let _ = write_json(
        &install.stage().join(FAILED),
        &Failed {
            version: version.to_owned(),
            reason: reason.to_owned(),
        },
    );
}

/// The top-level names in `dir`, the staging directory left out.
fn names_in(dir: &Path) -> io::Result<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(dir)? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if name != STAGE {
            names.insert(name);
        }
    }
    Ok(names)
}

impl Journal {
    fn path(install: &Install) -> PathBuf {
        install.stage().join(JOURNAL)
    }

    /// Plans the staged update and writes the journal before any rename.
    ///
    /// # Errors
    ///
    /// [`ApplyError::NothingStaged`], [`ApplyError::Plan`], or the write's.
    pub fn begin(install: &Install, from: &str) -> Result<Self, ApplyError> {
        let staged = staged(install).ok_or(ApplyError::NothingStaged)?;
        let new = names_in(&install.stage().join(NEW))?;
        let existing = names_in(&install.base)?;
        let steps =
            plan::plan(install.os, &new, &existing, &install.program).map_err(ApplyError::Plan)?;
        let journal = Self {
            from: from.to_owned(),
            to: staged.version,
            steps,
            done: 0,
            rolling_back: false,
        };
        write_json(&Self::path(install), &journal)?;
        Ok(journal)
    }

    /// Does the next step and records it. `false` once every step is done.
    ///
    /// # Errors
    ///
    /// The rename's or the journal write's; the step is then not recorded.
    pub fn step(&mut self, install: &Install) -> io::Result<bool> {
        let Some(step) = self.steps.get(self.done) else {
            return Ok(false);
        };
        let (from, to) = (install.at(&step.from), install.at(&step.to));
        if there(&to) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{} is in the way", to.display()),
            ));
        }
        fs::rename(&from, &to)?;
        sync_dir(to.parent().unwrap_or(&install.base));
        self.done += 1;
        write_json(&Self::path(install), self)?;
        Ok(true)
    }

    /// Whether step `index` is done on the disk: its source gone, its
    /// target there. Asked only of the one step after the recorded count,
    /// which is the only one a crash can have left unrecorded.
    fn looks_done(&self, install: &Install, index: usize) -> bool {
        self.steps
            .get(index)
            .is_some_and(|step| !there(&install.at(&step.from)) && there(&install.at(&step.to)))
    }

    /// Reverse steps durably; preserve all evidence on any failure.
    fn roll_back(&mut self, install: &Install) -> io::Result<()> {
        self.rolling_back = true;
        write_json(&Self::path(install), self)?;
        while self.done > 0 {
            let step = &self.steps[self.done - 1];
            let (from, to) = (install.at(&step.from), install.at(&step.to));
            if there(&to) && !there(&from) {
                // The staged tree may be gone (that is one reason to roll
                // back): its folder is made again to move the new entry
                // back into, so the old one's name is free.
                if let Some(parent) = from.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::rename(&to, &from)?;
                sync_dir(to.parent().unwrap_or(&install.base));
                sync_dir(from.parent().unwrap_or(&install.base));
            } else if !there(&from) || there(&to) {
                return Err(io::Error::other(
                    "rollback paths are ambiguous; retained journal",
                ));
            }
            self.done -= 1;
            write_json(&Self::path(install), self)?;
        }
        sync_dir(&install.base);
        fs::remove_file(Self::path(install))
    }

    /// Records the update as finished and clears the staging directory of
    /// everything but that record. Deleting what was set aside is tried
    /// now and again at the next start; on Windows the old program is still
    /// running now.
    fn finish(self, install: &Install) -> io::Result<Applied> {
        let applied = Applied {
            from: self.from,
            to: self.to,
            asides: plan::asides(&self.steps),
        };
        write_json(&install.stage().join(APPLIED), &applied)?;
        let stage = install.stage();
        let _ = fs::remove_file(stage.join(JOURNAL));
        let _ = fs::remove_file(stage.join(STAGED));
        let _ = fs::remove_file(stage.join(FAILED));
        let _ = remove_any(&stage.join(NEW));
        for aside in &applied.asides {
            let _ = remove_any(&install.at(aside));
        }
        Ok(applied)
    }
}

/// Installs the staged update. Called after the client's window closed.
///
/// On failure rollback is attempted. If a reverse step fails too, the
/// journal and all recovery files remain; [`recover`] must retry before any
/// new transaction. Native clients use [`crate::launch::activate`] instead.
///
/// # Errors
///
/// Why nothing was installed.
pub fn apply(install: &Install, from: &str) -> Result<Applied, ApplyError> {
    let _claim = Claim::take(&install.stage()).map_err(|err| {
        if err.kind() == io::ErrorKind::WouldBlock {
            ApplyError::Busy
        } else {
            ApplyError::Io(err)
        }
    })?;
    if Journal::path(install).exists() {
        return Err(ApplyError::Busy);
    }
    let version = staged(install).map(|s| s.version);
    let mut journal = match Journal::begin(install, from) {
        Ok(journal) => journal,
        Err(err) => {
            if let (Some(version), false) = (&version, matches!(err, ApplyError::NothingStaged)) {
                mark_failed(install, version, &err.to_string());
                let _ = remove_any(&install.stage().join(NEW));
                let _ = fs::remove_file(install.stage().join(STAGED));
            }
            return Err(err);
        }
    };
    loop {
        match journal.step(install) {
            Ok(true) => {}
            Ok(false) => break,
            Err(err) => {
                let to = journal.to.clone();
                journal.roll_back(install)?;
                mark_failed(install, &to, &err.to_string());
                let _ = remove_any(&install.stage().join(NEW));
                let _ = fs::remove_file(install.stage().join(STAGED));
                return Err(ApplyError::Io(err));
            }
        }
    }
    Ok(journal.finish(install)?)
}

/// What [`recover`] found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recovery {
    /// Another updater owns the installation, or recovery needs a retry.
    Deferred,
    /// No update was under way.
    Nothing,
    /// An interrupted update was finished.
    Finished(Applied),
    /// An interrupted update was undone; the old client is in place.
    RolledBack,
}

/// Finishes or undoes an update a crash interrupted. First thing at start,
/// before anything reads a file of the installation.
#[must_use]
pub fn recover(install: &Install) -> Recovery {
    let Ok(_claim) = Claim::take(&install.stage()) else {
        return Recovery::Deferred;
    };
    let Some(mut journal) = read_json::<Journal>(&Journal::path(install)) else {
        return if Journal::path(install).exists() {
            Recovery::Deferred
        } else {
            Recovery::Nothing
        };
    };
    journal.done = journal.done.min(journal.steps.len());
    if !journal.rolling_back
        && journal.done < journal.steps.len()
        && journal.looks_done(install, journal.done)
    {
        journal.done += 1;
    }
    // Finish when every remaining source is still there; a step that then
    // fails anyway (its target taken, a permission) undoes them all.
    let finishable =
        (journal.done..journal.steps.len()).all(|i| there(&install.at(&journal.steps[i].from)));
    if finishable && !journal.rolling_back {
        let mut ok = true;
        loop {
            match journal.step(install) {
                Ok(true) => {}
                Ok(false) => break,
                Err(_) => {
                    ok = false;
                    break;
                }
            }
        }
        if ok && let Ok(applied) = journal.clone().finish(install) {
            return Recovery::Finished(applied);
        }
    }
    let to = journal.to.clone();
    if journal.roll_back(install).is_err() {
        return Recovery::Deferred;
    }
    mark_failed(install, &to, "an interrupted update was undone");
    let _ = remove_any(&install.stage().join(NEW));
    let _ = fs::remove_file(install.stage().join(STAGED));
    Recovery::RolledBack
}

/// The finished update to announce, once: reads `applied.json`, deletes
/// what the old installation was set aside as (the running Windows program
/// among it, now that it no longer runs) and the record itself.
#[must_use]
pub fn take_applied(install: &Install) -> Option<Applied> {
    let _claim = Claim::take(&install.stage()).ok()?;
    let path = install.stage().join(APPLIED);
    let applied: Applied = read_json(&path)?;
    for aside in &applied.asides {
        let _ = remove_any(&install.at(aside));
    }
    let _ = fs::remove_file(&path);
    // Keep the directory and its permanent lock inode.
    Some(applied)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_windows_or_linux_install_is_the_programs_folder() {
        let exe =
            Path::new("/home/p/baylee-client-0.1.0-beta.2-x86_64-unknown-linux-gnu/baylee-client");
        let install = Install::around(exe, Os::Linux).unwrap();
        assert_eq!(install.base, exe.parent().unwrap());
        assert_eq!(install.program, "baylee-client");
    }

    #[test]
    fn a_macos_install_is_the_bundles_folder() {
        let exe = Path::new("/Applications/Baylee.app/Contents/MacOS/baylee-client");
        let install = Install::around(exe, Os::MacOs).unwrap();
        assert_eq!(install.base, Path::new("/Applications"));
        assert_eq!(install.program, "Baylee.app");
    }

    #[test]
    fn a_macos_program_outside_a_bundle_is_not_updated() {
        let exe = Path::new("/Users/p/src/baylee/target/release/baylee-client");
        assert_eq!(
            Install::around(exe, Os::MacOs),
            Err(Unplaceable::NotABundle)
        );
    }

    #[test]
    fn a_translocated_app_is_not_updated() {
        let exe = Path::new(
            "/private/var/folders/xy/T/AppTranslocation/1234-5678/d/Baylee.app/Contents/MacOS/baylee-client",
        );
        assert_eq!(
            Install::around(exe, Os::MacOs),
            Err(Unplaceable::Translocated)
        );
    }
}
