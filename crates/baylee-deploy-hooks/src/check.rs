//! What must hold before any hook runs, checked on open descriptors.
//!
//! Every directory from the base down to the hook directory is opened with
//! `O_DIRECTORY | O_NOFOLLOW` relative to the one before it and `fstat`ed:
//! a listed owner (root), and writable by nobody else (no group or other
//! write bit). Every hook is opened the same way from the directory's
//! descriptor, with `O_NOFOLLOW`, and must be a regular file of a listed
//! owner, not group- or other-writable, with its owner's execute bit. A
//! script's interpreter (`#!`) and the directories of the hooks' `PATH`
//! must be root's too. The descriptors are kept: [`crate::run`] executes
//! exactly the files checked here.

use std::fmt;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};

use rustix::fd::{AsFd, OwnedFd};
use rustix::fs::{FileType, Mode, OFlags, Stat};
use rustix::io::Errno;

/// A hook that passed every check: its place in the run (1-based), its
/// name (for the root-only log), and the open file that will run.
#[derive(Debug)]
pub struct Hook {
    /// 1-based, in byte order of the names.
    pub index: usize,
    /// The file name. Never printed where the deployer's log can see it.
    pub name: String,
    /// The file, opened `O_RDONLY | O_NOFOLLOW | O_CLOEXEC`.
    pub fd: OwnedFd,
}

/// Why nothing ran.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The dispatcher is not running as root.
    NotRoot,
    /// A directory on the way, the `PATH`, or an interpreter.
    Path {
        /// The path; always one of the fixed system paths or an
        /// interpreter's, never a hook's name.
        path: String,
        /// What is wrong with it.
        why: String,
    },
    /// A hook. `name` goes only to the root-only log.
    Hook {
        /// Its place in byte order, 1-based.
        index: usize,
        /// Of how many.
        count: usize,
        /// Its file name.
        name: String,
        /// What is wrong with it.
        why: String,
    },
}

impl Refusal {
    /// The same with the hook's name, for the root-only log.
    #[must_use]
    pub fn detail(&self) -> String {
        match self {
            Self::Hook {
                index,
                count,
                name,
                why,
            } => format!("hook {index} of {count} ({name}) {why}"),
            other => other.to_string(),
        }
    }
}

impl fmt::Display for Refusal {
    /// Neutral: no hook is named.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotRoot => f.write_str("not running as root"),
            Self::Path { path, why } => write!(f, "{path} {why}"),
            Self::Hook {
                index, count, why, ..
            } => write!(f, "hook {index} of {count} {why}"),
        }
    }
}

/// The flags every directory on the way is opened with.
const DIR: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);

/// What is wrong with a path's owner or mode, if anything.
fn ownership(stat: &Stat, owners: &[u32]) -> Option<String> {
    if !owners.contains(&stat.st_uid) {
        return Some(format!("is owned by uid {}, not root", stat.st_uid));
    }
    let mode = stat.st_mode & 0o7777;
    if mode & 0o022 != 0 {
        return Some(format!("is writable by group or others (mode {mode:04o})"));
    }
    None
}

/// Opens `base`, then each of `components` below it, checking every one.
/// `Ok(None)` when one of them does not exist: then there are no hooks.
///
/// # Errors
/// A [`Refusal::Path`] for a link, a non-directory, a foreign owner or a
/// group/other write bit anywhere on the way.
pub fn open_chain(
    base: &Path,
    components: &[&str],
    owners: &[u32],
) -> Result<Option<OwnedFd>, Refusal> {
    let mut shown = base.to_path_buf();
    let refuse = |path: &Path, why: String| Refusal::Path {
        path: path.display().to_string(),
        why,
    };
    let mut fd = rustix::fs::open(base, DIR, Mode::empty())
        .map_err(|e| refuse(&shown, format!("cannot be opened as a directory: {e}")))?;
    checked_dir(&fd, owners).map_err(|why| refuse(&shown, why))?;
    for component in components {
        shown.push(component);
        fd = match rustix::fs::openat(&fd, *component, DIR, Mode::empty()) {
            Ok(next) => next,
            Err(Errno::NOENT) => return Ok(None),
            Err(Errno::LOOP | Errno::NOTDIR) => {
                return Err(refuse(
                    &shown,
                    "is a symbolic link or not a directory".into(),
                ));
            }
            Err(e) => return Err(refuse(&shown, format!("cannot be opened: {e}"))),
        };
        checked_dir(&fd, owners).map_err(|why| refuse(&shown, why))?;
    }
    Ok(Some(fd))
}

/// A directory descriptor's owner and mode.
fn checked_dir(fd: impl AsFd, owners: &[u32]) -> Result<(), String> {
    let stat = rustix::fs::fstat(fd).map_err(|e| format!("cannot be read: {e}"))?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::Directory {
        return Err("is not a directory".into());
    }
    ownership(&stat, owners).map_or(Ok(()), Err)
}

/// The names in the hook directory that match [`crate::is_hook_name`], in
/// byte order.
///
/// # Errors
/// A [`Refusal::Path`] when the directory cannot be listed.
pub fn hook_names(dir: &OwnedFd, shown: &Path) -> Result<Vec<String>, Refusal> {
    let listing = rustix::fs::Dir::read_from(dir).map_err(|e| Refusal::Path {
        path: shown.display().to_string(),
        why: format!("cannot be listed: {e}"),
    })?;
    let mut names = Vec::new();
    for entry in listing {
        let entry = entry.map_err(|e| Refusal::Path {
            path: shown.display().to_string(),
            why: format!("cannot be listed: {e}"),
        })?;
        let name = entry.file_name().to_bytes();
        if crate::is_hook_name(name) {
            // The pattern is ASCII, so this never fails.
            names.push(String::from_utf8_lossy(name).into_owned());
        }
    }
    // `String`'s order is byte order: the C locale's.
    names.sort_unstable();
    Ok(names)
}

/// Opens and checks every hook in `names`; all of them, or a refusal.
///
/// # Errors
/// A [`Refusal::Hook`] for the first that is a link, not a regular file,
/// foreign, group/other-writable, not executable, or a script whose
/// interpreter is not root's; a [`Refusal::Path`] for an interpreter.
pub fn open_hooks(dir: &OwnedFd, names: &[String], owners: &[u32]) -> Result<Vec<Hook>, Refusal> {
    let count = names.len();
    let mut hooks = Vec::with_capacity(count);
    for (at, name) in names.iter().enumerate() {
        let index = at + 1;
        let refuse = |why: String| Refusal::Hook {
            index,
            count,
            name: name.clone(),
            why,
        };
        // `NONBLOCK`: a FIFO planted under a hook's name must not hang the
        // open; it is refused below as not a regular file.
        let flags =
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK | OFlags::NOCTTY;
        let fd = match rustix::fs::openat(dir, name.as_str(), flags, Mode::empty()) {
            Ok(fd) => fd,
            Err(Errno::LOOP) => return Err(refuse("is a symbolic link".into())),
            Err(e) => return Err(refuse(format!("cannot be opened: {e}"))),
        };
        let stat = rustix::fs::fstat(&fd).map_err(|e| refuse(format!("cannot be read: {e}")))?;
        if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
            return Err(refuse("is not a regular file".into()));
        }
        if let Some(why) = ownership(&stat, owners) {
            return Err(refuse(why));
        }
        if stat.st_mode & 0o100 == 0 {
            return Err(refuse("is not executable by its owner".into()));
        }
        if let Some(interpreter) = interpreter(&fd).map_err(refuse)? {
            real_path(&interpreter, owners, true)?;
        }
        hooks.push(Hook {
            index,
            name: name.clone(),
            fd,
        });
    }
    Ok(hooks)
}

/// A script's interpreter, from its `#!` line; `None` for anything else
/// (a binary).
fn interpreter(fd: &OwnedFd) -> Result<Option<PathBuf>, String> {
    let mut head = [0_u8; 256];
    let read = rustix::io::pread(fd, &mut head, 0).map_err(|e| format!("cannot be read: {e}"))?;
    let head = &head[..read];
    let Some(line) = head.strip_prefix(b"#!") else {
        return Ok(None);
    };
    let line = line.split(|b| *b == b'\n').next().unwrap_or_default();
    let program = line
        .split(|b| *b == b' ' || *b == b'\t')
        .find(|word| !word.is_empty())
        .ok_or("has an empty #! line")?;
    if program.first() != Some(&b'/') {
        return Err("names an interpreter by a relative path".into());
    }
    let program =
        std::str::from_utf8(program).map_err(|_| "names an interpreter that is not UTF-8")?;
    Ok(Some(PathBuf::from(program)))
}

/// Resolves `path` and checks the file and every directory above it: a
/// listed owner and no group/other write bit. With `file`, the last
/// component must be a regular file, else a directory.
///
/// Resolving follows links (`/bin` → `usr/bin` is ordinary) and then checks
/// what they lead to; this is a check of the system the hooks run on, not of
/// the hooks themselves, which are held open.
///
/// # Errors
/// A [`Refusal::Path`] naming the first component that fails.
pub fn real_path(path: &Path, owners: &[u32], file: bool) -> Result<(), Refusal> {
    let refuse = |at: &Path, why: String| Refusal::Path {
        path: at.display().to_string(),
        why,
    };
    let real = std::fs::canonicalize(path)
        .map_err(|e| refuse(path, format!("cannot be resolved: {e}")))?;
    let mut at = PathBuf::new();
    let mut components = real.components().peekable();
    while let Some(component) = components.next() {
        at.push(component);
        let meta = std::fs::symlink_metadata(&at)
            .map_err(|e| refuse(&at, format!("cannot be read: {e}")))?;
        let last = components.peek().is_none();
        let kind_ok = if last && file {
            meta.is_file()
        } else {
            meta.is_dir()
        };
        if !kind_ok {
            return Err(refuse(
                &at,
                "is not what it should be (file or directory)".into(),
            ));
        }
        if !owners.contains(&meta.uid()) {
            return Err(refuse(
                &at,
                format!("is owned by uid {}, not root", meta.uid()),
            ));
        }
        if meta.mode() & 0o022 != 0 {
            return Err(refuse(
                &at,
                format!(
                    "is writable by group or others (mode {:04o})",
                    meta.mode() & 0o7777
                ),
            ));
        }
    }
    Ok(())
}

/// Every directory of [`crate::HOOK_PATH`] that exists, checked as
/// [`real_path`] checks.
///
/// # Errors
/// The first [`Refusal::Path`].
pub fn search_path(owners: &[u32]) -> Result<(), Refusal> {
    for dir in crate::HOOK_PATH.split(':') {
        let dir = Path::new(dir);
        if std::fs::symlink_metadata(dir).is_ok() {
            real_path(dir, owners, false)?;
        }
    }
    Ok(())
}
