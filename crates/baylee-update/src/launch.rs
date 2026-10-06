//! Stable launch paths, immutable payloads, and atomic activation.
//!
//! The original package is never renamed or modified. Its launcher selects
//! a complete generation outside the bundle. A pending activation is replayed
//! before launching; failure leaves the previous generation launchable and
//! keeps the pending record for a later retry. Lock files are never unlinked.

use crate::apply::{self, Claim, Install, Staged};
use crate::plan::{NEW, Os};
use crate::translocation::{self, Status, Translocation};
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

/// [`selected`], unless the original package is at least as new as the
/// selected generation: a package the player installed by hand over the
/// original (a newer download, an installer) then starts instead of the
/// older update the launcher had installed. `original` is the package's
/// own version, which only its launcher knows (it is compiled into it); a
/// generation whose version does not parse keeps being selected.
///
/// # Errors
/// As [`selected`].
pub fn chosen(install: &Install, original: &semver::Version) -> io::Result<PathBuf> {
    let current = read::<Current>(&install.stage().join(CURRENT))?;
    let newer = current
        .as_ref()
        .is_some_and(|c| semver::Version::parse(&c.to).map_or(true, |to| to > *original));
    if newer {
        selected(install)
    } else {
        Ok(runtime(&install.base, install.os, &install.program))
    }
}

/// This launcher's version, which is its package's: the launcher is never
/// replaced by an update, so it is the version the package was released as.
fn package_version() -> semver::Version {
    semver::Version::parse(env!("CARGO_PKG_VERSION")).expect("the workspace version is semver")
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

/// Why a session may not install updates automatically.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Blocked {
    /// The original package's folder cannot be written by this user: an
    /// installation someone else manages (`/Applications` set up by an
    /// administrator, a root-owned `/opt`, a read-only volume).
    ReadOnly {
        /// The folder the probe was made in.
        folder: PathBuf,
        /// What the system answered.
        error: String,
    },
    /// macOS runs a translocated copy and would not say of what, so no
    /// state key would survive the next start.
    Translocated,
}

/// Where the original package really is, and whether this user's session
/// may install updates for it ([`placement`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placement {
    /// The original package (bundle or program), where it lies: under App
    /// Translocation the one the player unpacked, not the mount. `None`
    /// only when macOS would not say.
    pub original: Option<PathBuf>,
    /// Why automatic installation is off, or `None` when it is allowed.
    pub blocked: Option<Blocked>,
    /// Whether macOS runs it from a translocation mount.
    pub translocated: bool,
}

/// Where the package at `install` really is, and whether updates may be
/// installed for it.
///
/// Updates never write the original package (they live in the per-user
/// state directory), so its folder's writability is a policy, not a need:
/// a folder this user cannot write is someone else's to manage, and is
/// left to them. An app macOS translocates is one this user downloaded and
/// never moved, so it is theirs: its updates are allowed, keyed by the
/// original bundle ([`in_state_root_of`]), which survives the next start
/// where the randomised mount would not. Its folder is not probed: that is
/// usually `~/Downloads`, which macOS guards with a privacy prompt.
#[must_use]
pub fn placement(
    install: &Install,
    translocation: &dyn Translocation,
    appimage: Option<&Path>,
) -> Placement {
    let package = install.base.join(&install.program);
    let in_place = |package: PathBuf| {
        let folder = package.parent().unwrap_or(&install.base).to_path_buf();
        Placement {
            original: Some(package),
            blocked: probe(&folder).err().map(|err| Blocked::ReadOnly {
                folder,
                error: err.to_string(),
            }),
            translocated: false,
        }
    };
    // An AppImage runs from a fresh read-only mount each start; the file
    // the player keeps is the package, and its folder is what is probed.
    if let Some(image) = appimage.filter(|_| install.os == Os::Linux) {
        return in_place(image.to_path_buf());
    }
    if install.os != Os::MacOs {
        return in_place(package);
    }
    match translocation.status(&package) {
        Status::InPlace => in_place(package),
        Status::From(original) => Placement {
            original: Some(original),
            blocked: None,
            translocated: true,
        },
        Status::Unknown => Placement {
            original: None,
            blocked: Some(Blocked::Translocated),
            translocated: true,
        },
    }
}

/// The `AppImage` file this launcher runs from, if it runs from one.
///
/// The `AppImage` runtime mounts the image read-only at a fresh
/// `/tmp/.mount_…` each start and sets `$APPDIR` to that mount and
/// `$APPIMAGE` to the image file. Both are believed only together: the
/// launcher must lie inside `$APPDIR`, and `$APPIMAGE` must be an existing
/// file, so a stray variable in a shell cannot re-key a plain install.
/// `var` reads the environment (a fake in tests).
#[must_use]
pub fn appimage_of(
    install: &Install,
    var: &dyn Fn(&str) -> Option<std::ffi::OsString>,
) -> Option<PathBuf> {
    if install.os != Os::Linux {
        return None;
    }
    let image = PathBuf::from(var("APPIMAGE")?);
    let mount = PathBuf::from(var("APPDIR")?);
    let inside = |base: &Path, mount: &Path| base.starts_with(mount);
    let canonical = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    (image.is_absolute()
        && mount.is_absolute()
        && image.is_file()
        && (inside(&install.base, &mount) || inside(&canonical(&install.base), &canonical(&mount))))
    .then_some(image)
}

#[derive(Serialize, Deserialize)]
struct Session {
    install: Install,
    token: uuid::Uuid,
    /// Read by every runtime, the oldest included: keep it.
    writable: bool,
    /// The rest came after the launcher first shipped. A launcher that
    /// predates them sends none, and is permanent in its package, so a
    /// runtime reads their absence as "not said".
    #[serde(default)]
    blocked: Option<Blocked>,
    #[serde(default)]
    original: Option<PathBuf>,
    #[serde(default)]
    translocated: bool,
    /// The runtime this launcher started ([`chosen`]). A runtime checks it
    /// is that one; without it (an older launcher), against [`selected`],
    /// which is what such a launcher started.
    #[serde(default)]
    runtime: Option<PathBuf>,
}

/// A runtime's shared lifetime lease. Also survives an orphaned launcher.
pub struct ClientLease {
    _file: fs::File,
    writable: bool,
    blocked: Option<Blocked>,
    original: Option<PathBuf>,
    translocated: bool,
}

impl ClientLease {
    /// Whether automatic installation is permitted for this session.
    #[must_use]
    pub fn writable(&self) -> bool {
        self.writable
    }

    /// Why it is not, when the launcher said (one that predates this does not).
    #[must_use]
    pub fn blocked(&self) -> Option<&Blocked> {
        self.blocked.as_ref()
    }

    /// The original package, where it lies, when the launcher said.
    #[must_use]
    pub fn original(&self) -> Option<&Path> {
        self.original.as_deref()
    }

    /// Whether macOS runs the original from a translocation mount.
    #[must_use]
    pub fn translocated(&self) -> bool {
        self.translocated
    }
}

/// Put launcher state in a per-user root, isolated by canonical launch path.
///
/// # Errors
/// The installation path must exist and be canonicalizable.
pub fn in_state_root(install: &Install, root: &Path) -> io::Result<Install> {
    let path = fs::canonicalize(install.base.join(&install.program))?;
    Ok(keyed(install, root, &path))
}

/// [`in_state_root`], keyed by where the original package really is
/// ([`Placement::original`]): the same key whether macOS runs it in place
/// or from a translocation mount, so an update installed under one is
/// selected under the other. Without an original, the launch path keys it.
///
/// # Errors
/// As [`in_state_root`], when there is no original.
pub fn in_state_root_of(
    install: &Install,
    root: &Path,
    original: Option<&Path>,
) -> io::Result<Install> {
    match original {
        // An original that no longer resolves (moved since) still keys this
        // start by its name; the next start resolves the new place.
        Some(original) => Ok(keyed(
            install,
            root,
            &fs::canonicalize(original).unwrap_or_else(|_| original.to_path_buf()),
        )),
        None => in_state_root(install, root),
    }
}

fn keyed(install: &Install, root: &Path, package: &Path) -> Install {
    let digest = Sha256::digest(package.as_os_str().as_encoded_bytes());
    let mut managed = install.clone();
    managed.state = Some(root.join("baylee").join(crate::sign::hex_of(&digest)));
    managed
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

/// Whether this user can create (and remove) a file in `folder`.
fn probe(folder: &Path) -> io::Result<()> {
    let path = folder.join(format!(".baylee-probe-{}", uuid::Uuid::now_v7()));
    drop(
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?,
    );
    fs::remove_file(path)
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
    let started = match &session.runtime {
        Some(runtime) => runtime.clone(),
        None => selected(&session.install)?,
    };
    if token != Some(session.token)
        || fs::canonicalize(started)? != fs::canonicalize(std::env::current_exe()?)?
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
            blocked: session.blocked,
            original: session.original,
            translocated: session.translocated,
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
    let image = appimage_of(install, &|name| std::env::var_os(name));
    let place = placement(install, &translocation::System, image.as_deref());
    let managed;
    let install = if install.state.is_none() {
        managed = in_state_root_of(install, &state_root()?, place.original.as_deref())?;
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
        writable: place.blocked.is_none(),
        blocked: place.blocked,
        original: place.original,
        translocated: place.translocated,
        runtime: Some(chosen(install, &package_version())?),
    };
    let started = session
        .runtime
        .clone()
        .ok_or_else(|| io::Error::other("no runtime chosen"))?;
    apply::write_json(&install.stage().join("session.json"), &session.token)?;
    // Admission remains exclusive during conversion; a child of a killed
    // launcher must check its token under this same admission lock.
    live.unlock()?;
    live.try_lock_shared().map_err(io::Error::from)?;
    let mut child = Command::new(started)
        .args(args)
        .env(
            ENV,
            serde_json::to_string(&session).map_err(io::Error::other)?,
        )
        .spawn()?;
    drop(entry);
    child.wait()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::translocation::Fixed;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "baylee-launch-{tag}-{}-{}",
            std::process::id(),
            uuid::Uuid::now_v7()
        ));
        fs::create_dir_all(dir.join("Baylee.app")).unwrap();
        dir
    }

    fn mac(base: &Path) -> Install {
        Install {
            state: None,
            os: Os::MacOs,
            base: base.to_path_buf(),
            program: "Baylee.app".into(),
        }
    }

    #[test]
    fn an_app_in_a_writable_folder_installs_and_is_its_own_original() {
        let base = scratch("inplace");
        let place = placement(&mac(&base), &Fixed(Status::InPlace), None);
        assert_eq!(place.blocked, None);
        assert_eq!(place.original, Some(base.join("Baylee.app")));
        assert!(!place.translocated);
        fs::remove_dir_all(base).unwrap();
    }

    /// The owner's case of 06.10.2026: unpacked in Downloads, never moved,
    /// quarantined. The mount is read-only, the original's folder is not
    /// even looked at, and installing is allowed.
    #[test]
    fn a_translocated_app_installs_and_names_its_original() {
        let mount = Path::new("/private/var/folders/xy/T/AppTranslocation/1-2/d");
        let original = Path::new("/Users/p/Downloads/baylee-client-x/Baylee.app");
        let place = placement(&mac(mount), &Fixed(Status::From(original.into())), None);
        assert_eq!(
            place,
            Placement {
                original: Some(original.into()),
                blocked: None,
                translocated: true,
            }
        );
    }

    #[test]
    fn a_translocated_app_of_unknown_origin_does_not_install() {
        let mount = Path::new("/private/var/folders/xy/T/AppTranslocation/1-2/d");
        let place = placement(&mac(mount), &Fixed(Status::Unknown), None);
        assert_eq!(place.blocked, Some(Blocked::Translocated));
        assert_eq!(place.original, None);
    }

    /// The notice can say exactly which folder, and what the system said.
    #[cfg(unix)]
    #[test]
    fn a_read_only_folder_says_which_and_why() {
        use std::os::unix::fs::PermissionsExt as _;
        let base = scratch("readonly");
        fs::set_permissions(&base, fs::Permissions::from_mode(0o555)).unwrap();
        if fs::write(base.join("root-can"), "").is_ok() {
            // Running as root: nothing is read-only to it.
            fs::set_permissions(&base, fs::Permissions::from_mode(0o755)).unwrap();
            fs::remove_dir_all(base).unwrap();
            return;
        }
        let place = placement(&mac(&base), &Fixed(Status::InPlace), None);
        fs::set_permissions(&base, fs::Permissions::from_mode(0o755)).unwrap();
        let Some(Blocked::ReadOnly { folder, error }) = place.blocked else {
            panic!("not blocked: {:?}", place.blocked);
        };
        assert_eq!(folder, base);
        assert!(error.contains("ermission denied"), "{error}");
        fs::remove_dir_all(base).unwrap();
    }

    /// Other systems are never asked about translocation, even if a fake
    /// would say yes.
    #[test]
    fn only_macos_translocates() {
        let base = scratch("linux");
        let install = Install {
            os: Os::Linux,
            program: "baylee-client".into(),
            ..mac(&base)
        };
        fs::write(base.join("baylee-client"), "").unwrap();
        let place = placement(&install, &Fixed(Status::Unknown), None);
        assert_eq!(place.blocked, None);
        assert!(!place.translocated);
        fs::remove_dir_all(base).unwrap();
    }

    /// The state key: one per original, the same in place and translocated,
    /// and the same as before this change for an app that never was.
    #[test]
    fn the_state_follows_the_original_not_the_mount() {
        let base = scratch("key");
        let root = base.join("state");
        let install = mac(&base);
        let in_place = in_state_root(&install, &root).unwrap();
        let original = base.join("Baylee.app");
        let mounted = mac(Path::new("/private/var/folders/AppTranslocation/9/d"));
        let translocated = in_state_root_of(&mounted, &root, Some(&original)).unwrap();
        assert_eq!(translocated.state, in_place.state);
        assert_eq!(
            in_state_root_of(&install, &root, Some(&original))
                .unwrap()
                .state,
            in_place.state
        );
        let elsewhere = base.join("Elsewhere.app");
        fs::create_dir_all(&elsewhere).unwrap();
        assert_ne!(
            in_state_root_of(&install, &root, Some(&elsewhere))
                .unwrap()
                .state,
            in_place.state,
            "a moved package starts a state of its own"
        );
        fs::remove_dir_all(base).unwrap();
    }

    /// A launcher that predates the new fields (it is permanent in its
    /// package) still makes a session every runtime reads.
    #[test]
    fn an_old_launchers_session_reads() {
        let old = serde_json::json!({
            "install": mac(Path::new("/Applications")),
            "token": uuid::Uuid::now_v7(),
            "writable": false,
        });
        let session: Session = serde_json::from_value(old).unwrap();
        assert!(!session.writable);
        assert_eq!(session.blocked, None);
        assert_eq!(session.original, None);
        assert!(!session.translocated);
        assert_eq!(session.runtime, None);
    }

    /// An `AppImage`: the image file and its mount, as the `AppImage` runtime
    /// sets them, around a launcher at `<mount>/opt/baylee/baylee-client`.
    struct Image {
        root: PathBuf,
        file: PathBuf,
        install: Install,
    }

    fn image(tag: &str, mount: &str) -> Image {
        let root = scratch(tag);
        let file = root.join("Apps/Baylee-0.1.0-x86_64.AppImage");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, "ELF").unwrap();
        let base = root.join(mount).join("opt/baylee");
        fs::create_dir_all(&base).unwrap();
        let install = Install {
            state: None,
            os: Os::Linux,
            base,
            program: "baylee-client".into(),
        };
        Image {
            root,
            file,
            install,
        }
    }

    fn env<'a>(
        image: &'a Path,
        mount: &'a Path,
    ) -> impl Fn(&str) -> Option<std::ffi::OsString> + 'a {
        move |name| match name {
            "APPIMAGE" => Some(image.as_os_str().to_owned()),
            "APPDIR" => Some(mount.as_os_str().to_owned()),
            _ => None,
        }
    }

    /// Each start mounts the image somewhere new; the state stays one,
    /// keyed by the image file, and its folder decides about installing.
    #[test]
    fn an_appimage_is_keyed_by_its_file_not_its_mount() {
        let one = image("appimage", ".mount_aB1");
        let mount = one.root.join(".mount_aB1");
        let found = appimage_of(&one.install, &env(&one.file, &mount));
        assert_eq!(found.as_deref(), Some(one.file.as_path()));
        let place = placement(&one.install, &Fixed(Status::Unknown), found.as_deref());
        assert_eq!(place.original.as_deref(), Some(one.file.as_path()));
        assert_eq!(place.blocked, None, "a writable folder installs");
        assert!(!place.translocated);

        let state = one.root.join("state");
        let first = in_state_root_of(&one.install, &state, place.original.as_deref()).unwrap();
        let next = Install {
            base: one.root.join(".mount_Zq9/opt/baylee"),
            ..one.install.clone()
        };
        let second = in_state_root_of(&next, &state, Some(&one.file)).unwrap();
        assert_eq!(first.state, second.state, "a new mount, the same state");
        fs::remove_dir_all(one.root).unwrap();
    }

    #[test]
    fn appimage_variables_are_believed_only_together_and_from_inside() {
        let one = image("appimage-env", ".mount_aB1");
        let mount = one.root.join(".mount_aB1");
        let elsewhere = one.root.join(".mount_other");
        assert_eq!(
            appimage_of(&one.install, &env(&one.file, &elsewhere)),
            None,
            "not inside the mount"
        );
        assert_eq!(
            appimage_of(&one.install, &env(&one.root.join("gone.AppImage"), &mount)),
            None,
            "no such file"
        );
        assert_eq!(appimage_of(&one.install, &|_| None), None);
        let only_image = |name: &str| (name == "APPIMAGE").then(|| one.file.as_os_str().to_owned());
        assert_eq!(
            appimage_of(&one.install, &only_image),
            None,
            "both or neither"
        );
        let mac = Install {
            os: Os::MacOs,
            ..one.install.clone()
        };
        assert_eq!(appimage_of(&mac, &env(&one.file, &mount)), None);
        fs::remove_dir_all(one.root).unwrap();
    }

    /// An `AppImage` in a folder this user cannot write (`/opt`, say) says
    /// which folder, as any read-only package does.
    #[cfg(unix)]
    #[test]
    fn an_appimage_in_a_read_only_folder_names_it() {
        use std::os::unix::fs::PermissionsExt as _;
        let one = image("appimage-ro", ".mount_aB1");
        let apps = one.file.parent().unwrap().to_path_buf();
        fs::set_permissions(&apps, fs::Permissions::from_mode(0o555)).unwrap();
        let writable_anyway = fs::write(apps.join("root-can"), "").is_ok();
        let place = placement(&one.install, &Fixed(Status::InPlace), Some(&one.file));
        fs::set_permissions(&apps, fs::Permissions::from_mode(0o755)).unwrap();
        if !writable_anyway {
            let Some(Blocked::ReadOnly { folder, .. }) = place.blocked else {
                panic!("not blocked: {:?}", place.blocked);
            };
            assert_eq!(folder, apps);
        }
        fs::remove_dir_all(one.root).unwrap();
    }

    fn selecting(to: Option<&str>) -> (PathBuf, Install) {
        let base = scratch("chosen");
        let install = Install {
            state: Some(base.join("state")),
            os: Os::Linux,
            base: base.clone(),
            program: "baylee-client".into(),
        };
        fs::create_dir_all(install.stage()).unwrap();
        if let Some(to) = to {
            let current = Current {
                generation: uuid::Uuid::now_v7(),
                previous: None,
                from: "0.1.0-beta.4".into(),
                to: to.into(),
                announced: true,
            };
            apply::write_json(&install.stage().join(CURRENT), &current).unwrap();
        }
        (base, install)
    }

    /// A package installed by hand over the original, at least as new as
    /// the update the launcher had installed, is the one that starts.
    #[test]
    fn a_newer_or_equal_package_beats_an_installed_update() {
        let v = |s: &str| semver::Version::parse(s).unwrap();
        let (base, install) = selecting(Some("0.1.0-beta.6"));
        let original = base.join(runtime_name(Os::Linux));
        let generation = selected(&install).unwrap();
        assert_ne!(generation, original);
        assert_eq!(
            chosen(&install, &v("0.1.0-beta.5")).unwrap(),
            generation,
            "older package"
        );
        assert_eq!(
            chosen(&install, &v("0.1.0-beta.6")).unwrap(),
            original,
            "equal"
        );
        assert_eq!(
            chosen(&install, &v("0.1.0-beta.10")).unwrap(),
            original,
            "newer, by semver"
        );
        assert_eq!(
            chosen(&install, &v("0.1.0")).unwrap(),
            original,
            "a release after its betas"
        );
        fs::remove_dir_all(base).unwrap();

        let (base, install) = selecting(None);
        assert_eq!(
            chosen(&install, &v("0.1.0")).unwrap(),
            base.join(runtime_name(Os::Linux))
        );
        fs::remove_dir_all(base).unwrap();

        let (base, install) = selecting(Some("not a version"));
        assert_eq!(
            chosen(&install, &v("9.0.0")).unwrap(),
            selected(&install).unwrap()
        );
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn the_launcher_knows_its_packages_version() {
        assert_eq!(package_version().to_string(), env!("CARGO_PKG_VERSION"));
    }
}
