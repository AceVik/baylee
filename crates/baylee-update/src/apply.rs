//! Replacing the installation, under a journal.
//!
//! The steps are [`plan`](crate::plan::plan)'s renames. Before the first
//! one the whole list is written to `<base>/.baylee-update/journal.json`,
//! and after each one the count done is written again; every write goes to
//! a temporary file that is renamed over the journal, so the journal is
//! always one whole version or the other.
//!
//! A crash can land between a rename and the count that records it, so at
//! the next start ([`recover`]) the one step after the recorded count is
//! checked on the disk: if its source is gone and its target is there, it
//! happened. From there the update is **finished** when every remaining
//! step can still be done (its source there, its target free), and **rolled
//! back** otherwise, each done step renamed back. Both leave a program in
//! place: the new one, or the old one. Which of the two binaries runs the
//! recovery does not matter; both carry this code.
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

/// The staging directories an update is being staged into right now.
///
/// Claimed while an update is downloaded and unpacked, and asked for (never
/// waited for) before one is applied: a client closed in the middle of a
/// download leaves it unfinished rather than installing a tree that is
/// still being written. A staging directory belongs to one process, so a
/// process-wide set is the whole of it; per directory, so two tests in one
/// process do not wait on each other.
static STAGING: std::sync::Mutex<BTreeSet<PathBuf>> = std::sync::Mutex::new(BTreeSet::new());

/// A claim on one staging directory, released when dropped.
pub(crate) struct Claim(PathBuf);

impl Claim {
    /// Claims `stage`, or `None` while it is claimed already.
    pub(crate) fn take(stage: PathBuf) -> Option<Self> {
        let mut busy = STAGING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        busy.insert(stage.clone()).then_some(Self(stage))
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        STAGING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&self.0);
    }
}

const JOURNAL: &str = "journal.json";
const STAGED: &str = "staged.json";
const APPLIED: &str = "applied.json";
const FAILED: &str = "failed.json";

/// Where the running client is installed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Install {
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
    /// On macOS, a program that is not inside a `.app` bundle (a
    /// development build started by `cargo run`).
    NotABundle,
    /// macOS runs a quarantined app it was never allowed to move from a
    /// read-only copy (App Translocation), which cannot be replaced. The
    /// player moves `Baylee.app` once (to `/Applications`, say), or clears
    /// the quarantine as `README.txt` says, and it can.
    Translocated,
    /// A program path with no folder around it.
    NoFolder,
}

impl std::fmt::Display for Unplaceable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
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
        let name = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned());
        match os {
            Os::MacOs => {
                if exe.to_string_lossy().contains("/AppTranslocation/") {
                    return Err(Unplaceable::Translocated);
                }
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
                    os,
                    base: bundle.parent().ok_or(Unplaceable::NoFolder)?.to_path_buf(),
                    program: name(bundle).ok_or(Unplaceable::NotABundle)?,
                })
            }
            Os::Windows | Os::Linux => Ok(Self {
                os,
                base: exe.parent().ok_or(Unplaceable::NoFolder)?.to_path_buf(),
                program: name(exe).ok_or(Unplaceable::NoFolder)?,
            }),
        }
    }

    /// `<base>/.baylee-update`.
    #[must_use]
    pub fn stage(&self) -> PathBuf {
        self.base.join(STAGE)
    }

    fn at(&self, rel: &[String]) -> PathBuf {
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
fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    let mut file = fs::File::create(&tmp)?;
    file.write_all(&serde_json::to_vec_pretty(value).map_err(io::Error::other)?)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&tmp, path)?;
    sync_dir(path.parent().unwrap_or(Path::new(".")));
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

/// Makes a rename in `dir` durable. Best effort: a directory cannot be
/// opened for this on Windows, whose renames are durable when they return.
fn sync_dir(dir: &Path) {
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

    /// Renames every done step back, newest first. Best effort: a step that
    /// cannot be undone is left, and the rest are still tried.
    fn roll_back(&mut self, install: &Install) {
        while self.done > 0 {
            self.done -= 1;
            let step = &self.steps[self.done];
            let (from, to) = (install.at(&step.from), install.at(&step.to));
            if there(&to) && !there(&from) {
                // The staged tree may be gone (that is one reason to roll
                // back): its folder is made again to move the new entry
                // back into, so the old one's name is free.
                if let Some(parent) = from.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::rename(&to, &from);
            }
        }
        sync_dir(&install.base);
        let _ = fs::remove_file(Self::path(install));
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
/// On any failure every done step is renamed back, the version is recorded
/// as failed, and the old client is exactly as it was.
///
/// # Errors
///
/// Why nothing was installed.
pub fn apply(install: &Install, from: &str) -> Result<Applied, ApplyError> {
    let Some(_claim) = Claim::take(install.stage()) else {
        return Err(ApplyError::Busy);
    };
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
                journal.roll_back(install);
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
    let Some(mut journal) = read_json::<Journal>(&Journal::path(install)) else {
        return Recovery::Nothing;
    };
    journal.done = journal.done.min(journal.steps.len());
    if journal.done < journal.steps.len() && journal.looks_done(install, journal.done) {
        journal.done += 1;
    }
    // Finish when every remaining source is still there; a step that then
    // fails anyway (its target taken, a permission) undoes them all.
    let finishable =
        (journal.done..journal.steps.len()).all(|i| there(&install.at(&journal.steps[i].from)));
    if finishable {
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
    journal.roll_back(install);
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
    let path = install.stage().join(APPLIED);
    let applied: Applied = read_json(&path)?;
    for aside in &applied.asides {
        let _ = remove_any(&install.at(aside));
    }
    let _ = fs::remove_file(&path);
    // Nothing else is ours in there once an update finished; an empty
    // directory is removed, anything else is left for the next check.
    let _ = fs::remove_dir(install.stage());
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
