//! "Move Baylee to Applications": a copy of the running app where this user
//! may install updates, started in its place.
//!
//! Offered when updates may not be installed where the app is
//! ([`crate::launch::Blocked`]), and on macOS whenever it runs translocated.
//! The steps, each one checked before the next:
//!
//! 1. [`copy_bundle`] copies the running bundle into a hidden temporary
//!    beside the destination, keeping symlinks as links, file modes, and
//!    the code-signature seal (the signature lives in files of the bundle,
//!    copied byte for byte). The running bundle is the newest the player
//!    has: the original package, or the update generation the launcher
//!    selected, which carries its own launcher of the same release.
//! 2. [`strip_quarantine`] removes `com.apple.quarantine` from every entry
//!    of the copy: these are the player's own files, and without the flag
//!    macOS neither translocates the copy nor asks again.
//! 3. [`verify`] runs `codesign --verify --deep --strict` on the copy; a
//!    broken seal is removed, never started.
//! 4. The temporary is renamed into place, atomically, never over an
//!    existing app.
//!
//! Then [`relaunch`] opens the copy as a new instance and the client quits.
//! The original is never deleted by this: once the copy runs, it offers
//! [`SystemTrash`] for the old one, and the player may keep it.
//!
//! The copy's per-user state is new, keyed by its own path
//! ([`crate::launch::in_state_root`]): it needs nothing of the old one,
//! because it is a copy of the newest client the player had. What the old
//! state still holds (a download not yet installed) is fetched again.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Where the copy goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Destination {
    /// `~/Applications`: always this user's to write, no password asked.
    Home,
    /// `/Applications`, offered only where this user may write it.
    System,
}

impl Destination {
    /// The folder, under `home` for [`Destination::Home`].
    #[must_use]
    pub fn folder(self, home: &Path) -> PathBuf {
        match self {
            Self::Home => home.join("Applications"),
            Self::System => PathBuf::from("/Applications"),
        }
    }
}

/// A move that happened, kept until the copy has started and the player
/// has decided about the old one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Moved {
    /// The old package, where it lies (never a translocation mount).
    pub from: PathBuf,
    /// The copy.
    pub to: PathBuf,
}

/// The `.app` bundle around a program at `exe`, if it is in one.
#[must_use]
pub fn bundle_of(exe: &Path) -> Option<PathBuf> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let bundle = contents.parent()?;
    let named = |p: &Path, n: &str| p.file_name().is_some_and(|f| f == n);
    (named(macos, "MacOS")
        && named(contents, "Contents")
        && bundle
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("app")))
    .then(|| bundle.to_path_buf())
}

/// Whether this user can create an entry in the existing folder `dir`.
#[must_use]
pub fn can_write(dir: &Path) -> bool {
    let probe = dir.join(format!(".baylee-probe-{}", uuid::Uuid::now_v7()));
    fs::write(&probe, b"").is_ok() && fs::remove_file(probe).is_ok()
}

/// Copies the bundle at `source` into `folder` as `name`, unquarantined and
/// verified, and answers where it is. Nothing is left behind on failure,
/// and nothing is replaced: an existing `folder/name` is an error.
///
/// # Errors
/// [`io::ErrorKind::AlreadyExists`] when the name is taken; the copy's,
/// the quarantine removal's or the signature check's error otherwise.
pub fn copy_bundle(source: &Path, folder: &Path, name: &str) -> io::Result<PathBuf> {
    let to = folder.join(name);
    if fs::symlink_metadata(&to).is_ok() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} already exists", to.display()),
        ));
    }
    fs::create_dir_all(folder)?;
    let part = folder.join(format!(".{name}.baylee-move-{}", uuid::Uuid::now_v7()));
    let made = copy_tree(source, &part)
        .and_then(|()| strip_quarantine(&part))
        .and_then(|()| verify(&part))
        .and_then(|()| {
            // Re-checked: someone may have put one there meanwhile.
            if fs::symlink_metadata(&to).is_ok() {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("{} already exists", to.display()),
                ));
            }
            fs::rename(&part, &to)
        });
    if let Err(err) = made {
        let _ = fs::remove_dir_all(&part);
        return Err(err);
    }
    crate::apply::sync_dir(folder);
    Ok(to)
}

/// A copy of the tree at `from` as `to`: directories with their modes,
/// files with their bytes and modes (and, on macOS, their extended
/// attributes, as `fs::copy` clones them), symlinks as links.
fn copy_tree(from: &Path, to: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(from)?;
    if meta.file_type().is_symlink() {
        return symlink(&fs::read_link(from)?, to);
    }
    if meta.is_file() {
        fs::copy(from, to)?;
        return Ok(());
    }
    fs::create_dir(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        copy_tree(&entry.path(), &to.join(entry.file_name()))?;
    }
    // Last, so a read-only folder's mode does not stop its own filling.
    fs::set_permissions(to, meta.permissions())
}

#[cfg(unix)]
fn symlink(target: &Path, at: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, at)
}

#[cfg(not(unix))]
fn symlink(_target: &Path, at: &Path) -> io::Result<()> {
    Err(io::Error::other(format!(
        "{}: links are not copied on this system",
        at.display()
    )))
}

/// The attribute macOS sets on a download, and on what is unpacked from it.
pub const QUARANTINE: &str = "com.apple.quarantine";

/// Removes [`QUARANTINE`] from `root` and everything in it, never following
/// a link. Nothing to do on other systems.
///
/// # Errors
/// The first removal the system refused, other than "it had none".
pub fn strip_quarantine(root: &Path) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        xattr::remove(root, QUARANTINE)?;
        if fs::symlink_metadata(root)?.is_dir() {
            for entry in fs::read_dir(root)? {
                strip_quarantine(&entry?.path())?;
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = root;
    Ok(())
}

/// Whether `path` itself carries [`QUARANTINE`]; always `false` off macOS.
#[must_use]
pub fn quarantined(path: &Path) -> bool {
    #[cfg(target_os = "macos")]
    return xattr::has(path, QUARANTINE);
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        false
    }
}

/// Checks the bundle's code signature and every sealed resource
/// (`codesign --verify --deep --strict`). Nothing to check off macOS.
///
/// # Errors
/// What `codesign` said, when it said no.
pub fn verify(bundle: &Path) -> io::Result<()> {
    if !cfg!(target_os = "macos") {
        return Ok(());
    }
    let out = std::process::Command::new("/usr/bin/codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(bundle)
        .output()?;
    if out.status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "the copy's signature does not verify: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )))
    }
}

/// Opens `bundle` as a new instance (`open -n`), through Launch Services as
/// the Finder would. Only macOS has bundles to open.
///
/// # Errors
/// When `open` could not be run or said no.
pub fn relaunch(bundle: &Path) -> io::Result<()> {
    if !cfg!(target_os = "macos") {
        return Err(io::Error::other("only a macOS app is moved"));
    }
    let status = std::process::Command::new("/usr/bin/open")
        .arg("-n")
        .arg(bundle)
        .env_remove("BAYLEE_LAUNCH_SESSION")
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("open answered {status}")))
    }
}

/// Puts things into the Trash.
pub trait Trash {
    /// Moves `item` into the Trash and answers where it now is.
    ///
    /// # Errors
    /// Why the system would not.
    fn trash(&self, item: &Path) -> io::Result<PathBuf>;
}

/// The system's Trash: `NSFileManager`'s `trashItemAtURL:` on macOS, which
/// picks the volume's Trash and a free name, lets the Finder "Put Back",
/// and needs no Full Disk Access (a rename into `~/.Trash` does: that
/// folder is privacy-protected). No other system is moved from.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemTrash;

impl Trash for SystemTrash {
    fn trash(&self, item: &Path) -> io::Result<PathBuf> {
        #[cfg(target_os = "macos")]
        return foundation::trash(item);
        #[cfg(not(target_os = "macos"))]
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("{}: only a macOS app is moved", item.display()),
        ))
    }
}

#[cfg(target_os = "macos")]
mod foundation {
    //! `NSFileManager` through objc2-foundation's safe binding: a public
    //! Foundation API since macOS 10.8, so linked rather than looked up,
    //! and no `unsafe` is needed.

    use objc2_foundation::{NSFileManager, NSString, NSURL};
    use std::io;
    use std::path::{Path, PathBuf};

    pub(super) fn trash(item: &Path) -> io::Result<PathBuf> {
        let path = item
            .to_str()
            .ok_or_else(|| io::Error::other(format!("{} is not UTF-8", item.display())))?;
        let url = NSURL::fileURLWithPath(&NSString::from_str(path));
        let mut landed = None;
        NSFileManager::defaultManager()
            .trashItemAtURL_resultingItemURL_error(&url, Some(&mut landed))
            .map_err(|err| io::Error::other(err.localizedDescription().to_string()))?;
        landed
            .and_then(|url| url.path())
            .map(|path| PathBuf::from(path.to_string()))
            .ok_or_else(|| io::Error::other("the Trash did not say where it put it"))
    }
}

#[cfg(target_os = "macos")]
mod xattr {
    //! `removexattr`/`getxattr` from libSystem, without following links.

    use std::ffi::{CString, c_char, c_int, c_void};
    use std::io;
    use std::os::unix::ffi::OsStrExt as _;
    use std::path::Path;

    const XATTR_NOFOLLOW: c_int = 0x0001;
    const ENOATTR: i32 = 93;

    #[allow(unsafe_code)] // declarations of libSystem/CoreFoundation C functions
    unsafe extern "C" {
        fn removexattr(path: *const c_char, name: *const c_char, options: c_int) -> c_int;
        fn getxattr(
            path: *const c_char,
            name: *const c_char,
            value: *mut c_void,
            size: usize,
            position: u32,
            options: c_int,
        ) -> isize;
    }

    fn c(path: &Path) -> io::Result<CString> {
        CString::new(path.as_os_str().as_bytes()).map_err(io::Error::other)
    }

    /// Removes `name` from `path`; having none is fine.
    #[allow(unsafe_code)] // libSystem's xattr calls; no safe binding without a new dependency.
    pub(super) fn remove(path: &Path, name: &str) -> io::Result<()> {
        let (path, name) = (c(path)?, CString::new(name).map_err(io::Error::other)?);
        // SAFETY: both are NUL-terminated strings alive for the call;
        // removexattr reads them and writes nothing of ours.
        let rc = unsafe { removexattr(path.as_ptr(), name.as_ptr(), XATTR_NOFOLLOW) };
        if rc == 0 {
            return Ok(());
        }
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(ENOATTR) {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Whether `path` carries `name`.
    #[allow(unsafe_code)]
    pub(super) fn has(path: &Path, name: &str) -> bool {
        let (Ok(path), Ok(name)) = (c(path), CString::new(name)) else {
            return false;
        };
        // SAFETY: NUL-terminated strings alive for the call; a null value
        // with size 0 asks only for the size, so nothing is written.
        let size = unsafe {
            getxattr(
                path.as_ptr(),
                name.as_ptr(),
                std::ptr::null_mut(),
                0,
                0,
                XATTR_NOFOLLOW,
            )
        };
        size >= 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "baylee-relocate-{tag}-{}-{}",
            std::process::id(),
            uuid::Uuid::now_v7()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A bundle shaped like the release's: a launcher and a runtime in
    /// `MacOS`, resources, and `MacOS/assets` a relative link into them.
    /// On macOS with real programs (`/usr/bin/true`), signed ad hoc as
    /// `package-client.sh` signs the release: runtime first, then the
    /// bundle, so `verify` has a seal to check.
    fn bundle(at: &Path) -> PathBuf {
        let app = at.join("Baylee.app");
        let contents = app.join("Contents");
        fs::create_dir_all(contents.join("MacOS")).unwrap();
        fs::create_dir_all(contents.join("Resources/assets/fonts")).unwrap();
        fs::write(
            contents.join("Info.plist"),
            concat!(
                r#"<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict>"#,
                "<key>CFBundleIdentifier</key><string>local.baylee.test</string>",
                "<key>CFBundleExecutable</key><string>baylee-client</string>",
                "<key>CFBundlePackageType</key><string>APPL</string>",
                "</dict></plist>"
            ),
        )
        .unwrap();
        for exe in ["baylee-client", "baylee-runtime"] {
            let at = contents.join("MacOS").join(exe);
            if cfg!(target_os = "macos") {
                fs::copy("/usr/bin/true", &at).unwrap();
            } else {
                fs::write(&at, exe).unwrap();
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt as _;
                fs::set_permissions(&at, fs::Permissions::from_mode(0o755)).unwrap();
            }
        }
        fs::write(contents.join("Resources/assets/fonts/a.ttf"), "font").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("../Resources/assets", contents.join("MacOS/assets")).unwrap();
        if cfg!(target_os = "macos") {
            for what in [contents.join("MacOS/baylee-runtime"), app.clone()] {
                let signed = std::process::Command::new("/usr/bin/codesign")
                    .args(["--force", "--sign", "-"])
                    .arg(&what)
                    .output()
                    .unwrap();
                assert!(
                    signed.status.success(),
                    "{}",
                    String::from_utf8_lossy(&signed.stderr)
                );
            }
            verify(&app).unwrap();
        }
        app
    }

    #[test]
    fn a_bundle_is_found_around_its_program() {
        let exe = Path::new("/x/y/Baylee Beta.app/Contents/MacOS/baylee-runtime");
        assert_eq!(bundle_of(exe), Some(PathBuf::from("/x/y/Baylee Beta.app")));
        assert_eq!(
            bundle_of(Path::new("/x/target/release/baylee-client")),
            None
        );
        assert_eq!(
            Destination::Home.folder(Path::new("/Users/p")),
            Path::new("/Users/p/Applications")
        );
    }

    /// Not on Windows: the release's bundle holds a symlink, which only a
    /// Mac (and Linux, in this test) copies.
    #[cfg(unix)]
    #[test]
    fn the_copy_keeps_links_modes_and_bytes_and_leaves_no_temporary() {
        use std::os::unix::fs::PermissionsExt as _;
        let root = scratch("copy");
        let source = bundle(&root.join("Downloads"));
        let apps = root.join("Applications");
        let to = copy_bundle(&source, &apps, "Baylee.app").unwrap();
        assert_eq!(to, apps.join("Baylee.app"));
        let link = to.join("Contents/MacOS/assets");
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            fs::read_link(&link).unwrap(),
            Path::new("../Resources/assets")
        );
        assert_eq!(
            fs::read_to_string(link.join("fonts/a.ttf")).unwrap(),
            "font"
        );
        let mode = fs::metadata(to.join("Contents/MacOS/baylee-runtime"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755);
        let names: Vec<_> = fs::read_dir(&apps)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["Baylee.app"], "no temporary is left");
        assert!(
            source.join("Contents/MacOS/baylee-runtime").is_file(),
            "the original stays"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn an_existing_app_is_never_replaced() {
        let root = scratch("taken");
        let source = bundle(&root.join("Downloads"));
        let apps = root.join("Applications");
        fs::create_dir_all(apps.join("Baylee.app")).unwrap();
        fs::write(apps.join("Baylee.app/mine"), "player's").unwrap();
        let err = copy_bundle(&source, &apps, "Baylee.app").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(
            fs::read_to_string(apps.join("Baylee.app/mine")).unwrap(),
            "player's"
        );
        assert_eq!(
            fs::read_dir(&apps).unwrap().count(),
            1,
            "no temporary is left"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_failed_copy_leaves_nothing() {
        let root = scratch("fail");
        let apps = root.join("Applications");
        let err = copy_bundle(&root.join("missing.app"), &apps, "Baylee.app").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert_eq!(fs::read_dir(&apps).unwrap().count(), 0);
        fs::remove_dir_all(root).unwrap();
    }

    /// The copy carries no quarantine although the original did, on the
    /// bundle and inside it, and the original keeps its own flag.
    #[cfg(target_os = "macos")]
    #[test]
    fn the_copy_is_not_quarantined_and_its_seal_verifies() {
        let root = scratch("quarantine");
        let source = bundle(&root.join("Downloads"));
        let flag = "0083;00000000;Safari;";
        for path in [
            source.clone(),
            source.join("Contents/MacOS/baylee-runtime"),
            source.join("Contents/Resources/assets/fonts/a.ttf"),
        ] {
            let set = std::process::Command::new("/usr/bin/xattr")
                .args(["-w", QUARANTINE, flag])
                .arg(&path)
                .status()
                .unwrap();
            assert!(set.success());
            assert!(quarantined(&path), "{}", path.display());
        }
        let to = copy_bundle(&source, &root.join("Applications"), "Baylee.app").unwrap();
        for rel in [
            "",
            "Contents/MacOS/baylee-runtime",
            "Contents/Resources/assets/fonts/a.ttf",
        ] {
            let path = if rel.is_empty() {
                to.clone()
            } else {
                to.join(rel)
            };
            assert!(
                !quarantined(&path),
                "{} is still quarantined",
                path.display()
            );
        }
        assert!(quarantined(&source), "the original is not touched");
        verify(&to).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    /// A copy whose seal is broken is refused and removed.
    #[cfg(target_os = "macos")]
    #[test]
    fn a_broken_seal_is_refused() {
        let root = scratch("seal");
        let source = bundle(&root.join("Downloads"));
        fs::write(
            source.join("Contents/Resources/assets/fonts/a.ttf"),
            "changed",
        )
        .unwrap();
        let apps = root.join("Applications");
        assert!(copy_bundle(&source, &apps, "Baylee.app").is_err());
        assert_eq!(fs::read_dir(&apps).unwrap().count(), 0);
        fs::remove_dir_all(root).unwrap();
    }

    /// For a live check on a Mac, by hand: copies a real packaged bundle
    /// (a translocation mount of it, say) as the client's button does.
    /// `BAYLEE_LIVE_SOURCE=<bundle> BAYLEE_LIVE_FOLDER=<folder> cargo test
    /// -p baylee-update --lib live_copy -- --ignored --nocapture`
    #[test]
    #[ignore = "live check helper"]
    fn live_copy() {
        let (Some(source), Some(folder)) = (
            std::env::var_os("BAYLEE_LIVE_SOURCE"),
            std::env::var_os("BAYLEE_LIVE_FOLDER"),
        ) else {
            return;
        };
        let started = std::time::Instant::now();
        let to = copy_bundle(Path::new(&source), Path::new(&folder), "Baylee.app").unwrap();
        println!("copied to {} in {:?}", to.display(), started.elapsed());
    }

    #[test]
    fn stripping_what_has_no_quarantine_is_fine() {
        let root = scratch("plain");
        fs::write(root.join("f"), "x").unwrap();
        strip_quarantine(&root).unwrap();
        assert!(!quarantined(&root.join("f")));
        fs::remove_dir_all(root).unwrap();
    }

    /// The real Trash, through Foundation: the item is gone from where it
    /// was and lies where the answer says. What it put there is removed
    /// again at once (a uniquely named empty folder of this test's own).
    #[cfg(target_os = "macos")]
    #[test]
    fn the_system_trash_takes_an_item_and_says_where() {
        let root = scratch("trash");
        let item = root.join(format!("baylee-trash-test-{}.app", uuid::Uuid::now_v7()));
        fs::create_dir_all(&item).unwrap();
        let landed = SystemTrash.trash(&item).unwrap();
        assert!(!item.exists(), "it left where it was");
        assert!(
            landed.is_dir(),
            "it is where the answer says: {}",
            landed.display()
        );
        assert_eq!(landed.file_name(), item.file_name());
        fs::remove_dir(&landed).unwrap();
        assert!(SystemTrash.trash(&item).is_err(), "nothing there, an error");
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn no_other_system_trashes() {
        let err = SystemTrash.trash(Path::new("/tmp/x")).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Unsupported);
    }
}
