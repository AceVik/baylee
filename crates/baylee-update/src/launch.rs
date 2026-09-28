//! Stable launch paths, immutable payloads, and atomic activation.
//!
//! The original package is never renamed or modified. Its launcher selects
//! a complete generation outside the bundle. A pending activation is replayed
//! before launching; failure leaves the previous generation launchable and
//! keeps the pending record for a later retry. Lock files are never unlinked.

use crate::apply::{self, Claim, Install, Staged};
use crate::plan::{NEW, Os};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

const ENV: &str = "BAYLEE_LAUNCH_SESSION";
const CURRENT: &str = "current.json";
const PENDING: &str = "activation.json";

/// The real client name; the public executable name belongs to the launcher.
#[must_use]
pub fn runtime_name(os: Os) -> &'static str {
    if os == Os::Windows {
        "baylee-runtime.exe"
    } else {
        "baylee-runtime"
    }
}

fn runtime(root: &Path, os: Os, bundle: &str) -> PathBuf {
    if os == Os::MacOs {
        root.join(bundle)
            .join("Contents/MacOS")
            .join(runtime_name(os))
    } else {
        root.join(runtime_name(os))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Current {
    generation: uuid::Uuid,
    #[serde(default)]
    previous: Option<uuid::Uuid>,
    from: String,
    to: String,
    announced: bool,
}

fn read<T: serde::de::DeserializeOwned>(path: &Path) -> io::Result<Option<T>> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(io::Error::other),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

fn generation(install: &Install, current: &Current) -> PathBuf {
    install
        .stage()
        .join("versions")
        .join(current.generation.to_string())
}

/// Flush every regular file and directory before publishing its pointer.
/// Symlinks are kept as links, never followed outside the payload.
fn sync_tree(root: &Path) -> io::Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            sync_tree(&entry.path())?;
        } else if kind.is_file() {
            // FlushFileBuffers on Windows requires a writable handle.
            fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(entry.path())?
                .sync_all()?;
        }
    }
    apply::sync_dir(root);
    Ok(())
}

fn move_payload(install: &Install, next: &Current) -> io::Result<()> {
    let dest = generation(install, next);
    if !dest.exists() {
        let new = install.stage().join(NEW);
        if !runtime(&new, install.os, install.os.program()).is_file() {
            return Err(io::Error::other(
                "staged release has no runtime; activation retained",
            ));
        }
        sync_tree(&new)?;
        fs::create_dir_all(
            dest.parent()
                .ok_or_else(|| io::Error::other("no versions folder"))?,
        )?;
        fs::rename(new, &dest)?;
        apply::sync_dir(dest.parent().unwrap_or(&install.base));
        apply::sync_dir(&install.stage());
    }
    if !runtime(&dest, install.os, install.os.program()).is_file() {
        return Err(io::Error::other(
            "activation payload is incomplete; previous client retained",
        ));
    }
    Ok(())
}

fn complete(install: &Install, next: &Current) -> io::Result<()> {
    move_payload(install, next)?;
    apply::write_json(&install.stage().join(CURRENT), next)?;
    // CURRENT is the commit. Cleanup is idempotent and must not undo it.
    fs::remove_file(install.stage().join(PENDING))?;
    let _ = fs::remove_file(install.stage().join("staged.json"));
    apply::sync_dir(&install.stage());
    Ok(())
}

fn recover_locked(install: &Install) -> io::Result<()> {
    if let Some(next) = read::<Current>(&install.stage().join(PENDING))? {
        complete(install, &next)?;
    }
    Ok(())
}

/// Activate a verified staged release without replacing any running file.
///
/// # Errors
/// Returns lock, validation or I/O failures. Pending activation remains
/// retryable; the previous pointer and the original package remain usable.
pub fn activate(install: &Install, from: &str) -> io::Result<String> {
    Activation::begin(install, from)?.commit()
}

/// A claimed, durable activation. Dropping it at any point leaves a
/// transaction the permanent launcher can resume, with the old pointer intact.
pub struct Activation {
    install: Install,
    next: Current,
    _claim: Claim,
}

impl Activation {
    /// Validate staging and durably record intent before moving anything.
    ///
    /// # Errors
    /// Lock, pending recovery, staged payload, or record write failures.
    pub fn begin(install: &Install, from: &str) -> io::Result<Self> {
        let claim = Claim::take(&install.stage())?;
        recover_locked(install)?;
        let staged: Staged =
            apply::staged(install).ok_or_else(|| io::Error::other("no update staged"))?;
        let next = Current {
            generation: uuid::Uuid::now_v7(),
            from: from.into(),
            to: staged.version,
            announced: false,
            previous: read::<Current>(&install.stage().join(CURRENT))?.map(|c| c.generation),
        };
        if !runtime(&install.stage().join(NEW), install.os, install.os.program()).is_file() {
            return Err(io::Error::other(
                "release predates the stable launcher; install it manually",
            ));
        }
        apply::write_json(&install.stage().join(PENDING), &next)?;
        Ok(Self {
            install: install.clone(),
            next,
            _claim: claim,
        })
    }

    /// Durably place the payload, without changing the selected version.
    ///
    /// # Errors
    /// File flush or rename failures; all recovery information is retained.
    pub fn place(&self) -> io::Result<()> {
        move_payload(&self.install, &self.next)
    }

    /// Atomically select the payload, then remove the completed intent.
    ///
    /// # Errors
    /// Placement, pointer-write or cleanup failures; safe to recover again.
    pub fn commit(self) -> io::Result<String> {
        complete(&self.install, &self.next)?;
        Ok(self.next.to)
    }
}

/// Read the selected runtime, or the untouched original on first launch.
///
/// # Errors
/// Invalid pointer records are reported instead of silently selecting a
/// different version. This function does not perform recovery.
pub fn selected(install: &Install) -> io::Result<PathBuf> {
    Ok(match read::<Current>(&install.stage().join(CURRENT))? {
        Some(current) => runtime(
            &generation(install, &current),
            install.os,
            install.os.program(),
        ),
        None => runtime(&install.base, install.os, &install.program),
    })
}

/// Announce a committed activation once, without deleting a payload.
///
/// # Errors
/// Lock or record I/O failures. An unsuccessful write leaves it unannounced.
pub fn take_updated(install: &Install) -> io::Result<Option<String>> {
    let _claim = Claim::take(&install.stage())?;
    let Some(mut current) = read::<Current>(&install.stage().join(CURRENT))? else {
        return Ok(None);
    };
    if current.announced {
        return Ok(None);
    }
    current.announced = true;
    apply::write_json(&install.stage().join(CURRENT), &current)?;
    Ok(Some(current.to))
}

#[derive(Serialize, Deserialize)]
struct Session {
    install: Install,
    token: uuid::Uuid,
    writable: bool,
}

/// A runtime's shared lifetime lease. Also survives an orphaned launcher.
pub struct ClientLease {
    _file: fs::File,
    writable: bool,
}

impl ClientLease {
    /// Whether automatic installation is permitted for this session.
    #[must_use]
    pub fn writable(&self) -> bool {
        self.writable
    }
}

/// Put launcher state in a per-user root, isolated by canonical launch path.
///
/// # Errors
/// The installation path must exist and be canonicalizable.
pub fn in_state_root(install: &Install, root: &Path) -> io::Result<Install> {
    let path = fs::canonicalize(install.base.join(&install.program))?;
    let digest = Sha256::digest(path.as_os_str().as_encoded_bytes());
    let mut managed = install.clone();
    managed.state = Some(root.join("baylee").join(crate::sign::hex_of(&digest)));
    Ok(managed)
}

fn state_root() -> io::Result<PathBuf> {
    let root = if cfg!(target_os = "windows") {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state"))
            })
    };
    root.filter(|p| p.is_absolute())
        .ok_or_else(|| io::Error::other("no absolute per-user state directory"))
}

fn original_writable(install: &Install) -> bool {
    if install.os == Os::MacOs
        && install
            .base
            .to_string_lossy()
            .contains("/AppTranslocation/")
    {
        return false;
    }
    let path = install
        .base
        .join(format!(".baylee-probe-{}", uuid::Uuid::now_v7()));
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(file) => {
            drop(file);
            fs::remove_file(path).is_ok()
        }
        Err(_) => false,
    }
}

// Called only while the launcher holds the EXCLUSIVE lifetime lock. Never
// remove a generation while an orphaned runtime or another launcher uses it.
fn prune(install: &Install) -> io::Result<()> {
    let Some(current) = read::<Current>(&install.stage().join(CURRENT))? else {
        return Ok(());
    };
    for entry in fs::read_dir(install.stage().join("versions"))? {
        let entry = entry?;
        let Ok(id) = uuid::Uuid::parse_str(&entry.file_name().to_string_lossy()) else {
            continue;
        };
        if id != current.generation && Some(id) != current.previous && entry.file_type()?.is_dir() {
            fs::remove_dir_all(entry.path())?;
        }
    }
    Ok(())
}

fn lifetime(install: &Install) -> io::Result<fs::File> {
    fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(install.stage().join("lifetime.lock"))
}

/// Join the launcher session before the updater starts. Directly launched
/// runtimes have no session and must not install updates automatically.
///
/// # Errors
/// A superseded session, wrong executable, or lock failure aborts startup.
pub fn join() -> io::Result<Option<(Install, ClientLease)>> {
    let Some(value) = std::env::var_os(ENV) else {
        return Ok(None);
    };
    let session: Session =
        serde_json::from_str(&value.to_string_lossy()).map_err(io::Error::other)?;
    let _entry = Claim::wait(&session.install.stage(), "entry.lock")?;
    let token: Option<uuid::Uuid> = read(&session.install.stage().join("session.json"))?;
    if token != Some(session.token)
        || fs::canonicalize(selected(&session.install)?)?
            != fs::canonicalize(std::env::current_exe()?)?
    {
        return Err(io::Error::other(
            "launcher session was superseded or names another runtime",
        ));
    }
    let file = lifetime(&session.install)?;
    file.try_lock_shared().map_err(io::Error::from)?;
    Ok(Some((
        session.install,
        ClientLease {
            _file: file,
            writable: session.writable,
        },
    )))
}

/// Run one client through its permanent launch path. A second launch is
/// refused while either the launcher or its child still holds a lease.
///
/// # Errors
/// Installation, process-spawn, lock and pointer failures.
pub fn run(
    install: &Install,
    args: impl IntoIterator<Item = std::ffi::OsString>,
) -> io::Result<ExitStatus> {
    let managed;
    let install = if install.state.is_none() {
        managed = in_state_root(install, &state_root()?)?;
        &managed
    } else {
        install
    };
    let entry = Claim::named(&install.stage(), "entry.lock")?;
    let live = lifetime(install)?;
    live.try_lock().map_err(io::Error::from)?;
    {
        let _mutation = Claim::take(&install.stage())?;
        if let Err(err) = recover_locked(install) {
            eprintln!("Baylee: activation deferred; starting the previous client: {err}");
        } else if let Err(err) = prune(install) {
            eprintln!("Baylee: old payload cleanup deferred: {err}");
        }
    }
    let session = Session {
        install: install.clone(),
        token: uuid::Uuid::now_v7(),
        writable: original_writable(install),
    };
    apply::write_json(&install.stage().join("session.json"), &session.token)?;
    // Admission remains exclusive during conversion; a child of a killed
    // launcher must check its token under this same admission lock.
    live.unlock()?;
    live.try_lock_shared().map_err(io::Error::from)?;
    let mut child = Command::new(selected(install)?)
        .args(args)
        .env(
            ENV,
            serde_json::to_string(&session).map_err(io::Error::other)?,
        )
        .spawn()?;
    drop(entry);
    child.wait()
}
