//! Starting the client again once it has quit: "Restart now" when an update
//! is ready (`docs/client.md` §"Restarting into an update").
//!
//! The update is installed as the program ends (`launch::activate`, from the
//! runtime's exit), and only a launcher started *after* that selects it. A
//! launcher started while the old runtime or its launcher still holds its
//! lifetime lease refuses outright ("another instance may be running"), and
//! the launcher is permanent in every package, so an older one cannot be
//! taught to wait. So the runtime leaves a **helper** behind: its own
//! executable, run with [`FLAG`], which
//!
//! 1. reads its stdin to the end: the runtime writes the handoff (at most
//!    [`HANDOFF_BYTES`]) and keeps the pipe open until it exits, so the end
//!    of the input *is* the old process ending, on every system, without a
//!    pid to poll;
//! 2. waits until the installation's lifetime lock is free, which is the old
//!    launcher gone as well ([`Again::lifetime`], none for a runtime started
//!    without one), at most [`WAIT`];
//! 3. starts [`Again::program`] with [`Again::args`] and the handoff on its
//!    stdin, and exits without waiting for it.
//!
//! **The handoff is a secret** (the client's sign-in, for a game resumed at
//! a gateway), and so it travels only through pipes: never in an argument,
//! the environment, a file or a log. A launcher hands its stdin to the
//! runtime it starts (`launch::run` leaves it inherited), every launcher
//! ever shipped included. What it is and how it is read is the client's
//! business; this module carries bytes.

use crate::apply::Install;
use crate::plan::Os;
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs;
use std::io::{self, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::{ChildStdin, Command, Stdio};
use std::time::{Duration, Instant};

/// The argument that makes the client's executable the helper. The next
/// argument is the [`Again`], as JSON: paths and flags, nothing secret.
pub const FLAG: &str = "--baylee-relaunch";

/// The most a handoff may be.
pub const HANDOFF_BYTES: usize = 16 * 1024;

/// The launcher's file name inside a macOS bundle (`Contents/MacOS`), which
/// `CFBundleExecutable` names (`docs/releasing.md` §"Desktop launcher and
/// recovery").
pub const MAC_LAUNCHER: &str = "baylee-client";

/// How long the helper waits for the old client and its launcher to end.
pub const WAIT: Duration = Duration::from_secs(60);

/// What to start once the old client is gone.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Again {
    /// The program: the package's launcher, or the runtime itself when no
    /// launcher started it (a development build).
    pub program: PathBuf,
    /// Its arguments.
    pub args: Vec<OsString>,
    /// The installation's lifetime lock to wait for, when a launcher
    /// started the client.
    pub lifetime: Option<PathBuf>,
}

impl Again {
    /// The launcher of `install` (`original` is where the package really is,
    /// as the launcher's session said), starting with `args`: what a player
    /// starting Baylee again would start.
    ///
    /// An `AppImage` is started as its image file, because the folder
    /// `install` names is the image's mount, which goes with the old
    /// process. Everywhere else the launcher is the package's own
    /// executable, under macOS App Translocation the mount's (which stays
    /// until the user logs out), so a quarantined app is not assessed again.
    #[must_use]
    pub fn launcher(install: &Install, original: Option<&Path>, args: Vec<OsString>) -> Self {
        let program = match install.os {
            Os::MacOs => install
                .base
                .join(&install.program)
                .join("Contents/MacOS")
                .join(MAC_LAUNCHER),
            Os::Linux => original
                .filter(|image| !image.starts_with(&install.base) && image.is_file())
                .map_or_else(|| install.base.join(&install.program), Path::to_path_buf),
            Os::Windows => install.base.join(&install.program),
        };
        Self {
            program,
            args,
            lifetime: Some(install.stage().join("lifetime.lock")),
        }
    }

    /// The runtime `exe` itself, when no launcher started it.
    #[must_use]
    pub fn direct(exe: &Path, args: Vec<OsString>) -> Self {
        Self {
            program: exe.to_path_buf(),
            args,
            lifetime: None,
        }
    }
}

/// The helper's door, held open until this process ends: dropping it
/// before then starts the client again while this one still runs.
pub struct Handoff {
    _pipe: ChildStdin,
}

/// Starts the helper as `helper` (the client's own executable, or a test's)
/// with [`FLAG`] and `again`, writes `handoff` to it, and hands back the
/// open pipe, which the caller keeps until the process ends.
///
/// # Errors
/// A handoff too long, the helper not starting, or the write failing.
pub fn leave_behind(mut helper: Command, again: &Again, handoff: &[u8]) -> io::Result<Handoff> {
    if handoff.len() > HANDOFF_BYTES {
        return Err(io::Error::other("the handoff is too long"));
    }
    helper
        .arg(FLAG)
        .arg(serde_json::to_string(again).map_err(io::Error::other)?)
        .env_remove("BAYLEE_LAUNCH_SESSION")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    no_window(&mut helper);
    let mut child = helper.spawn()?;
    let mut pipe = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("the helper has no stdin"))?;
    pipe.write_all(handoff)?;
    pipe.flush()?;
    // The child is not waited for: it outlives this process by design.
    drop(child);
    Ok(Handoff { _pipe: pipe })
}

/// The [`Again`] after [`FLAG`] in `args`, when this process is the helper.
#[must_use]
pub fn asked(args: &[OsString]) -> Option<Again> {
    let at = args.iter().position(|a| a == FLAG)?;
    serde_json::from_str(&args.get(at + 1)?.to_string_lossy()).ok()
}

/// The helper's whole life ([`FLAG`]): reads the handoff from `input` to its
/// end, waits for `again`'s lifetime lock at most `wait`, and starts it.
///
/// # Errors
/// An input longer than [`HANDOFF_BYTES`], a lock that stays held, or the
/// program not starting.
pub fn helper(input: impl io::Read, again: &Again, wait: Duration) -> io::Result<()> {
    let mut handoff = Vec::new();
    input
        .take(HANDOFF_BYTES as u64 + 1)
        .read_to_end(&mut handoff)?;
    if handoff.len() > HANDOFF_BYTES {
        return Err(io::Error::other("the handoff is too long"));
    }
    if let Some(lifetime) = &again.lifetime {
        wait_free(lifetime, wait)?;
    }
    let mut command = Command::new(&again.program);
    command
        .args(&again.args)
        .env_remove("BAYLEE_LAUNCH_SESSION")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    no_window(&mut command);
    let mut child = command.spawn()?;
    if let Some(mut pipe) = child.stdin.take() {
        // A client that never reads it does not hold this up: the handoff
        // fits a pipe's buffer, and the pipe closes when this returns.
        let _ = pipe.write_all(&handoff);
    }
    Ok(())
}

/// The handoff a client started again by [`helper`] finds on its stdin: read
/// only when its stdin is not a terminal (a `--resume` typed by hand would
/// otherwise wait on the keyboard), at most [`HANDOFF_BYTES`], and for at
/// most `within` (a pipe nobody closes gives nothing rather than a client
/// that never opens). `None` when there is nothing.
#[must_use]
pub fn handoff_from_stdin(within: Duration) -> Option<Vec<u8>> {
    use std::io::IsTerminal as _;
    if io::stdin().is_terminal() {
        return None;
    }
    let (sent, got) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let read = io::stdin()
            .lock()
            .take(HANDOFF_BYTES as u64 + 1)
            .read_to_end(&mut bytes);
        let _ = sent.send(read.ok().map(|_| bytes));
    });
    got.recv_timeout(within)
        .ok()
        .flatten()
        .filter(|bytes| !bytes.is_empty() && bytes.len() <= HANDOFF_BYTES)
}

/// Waits until nobody holds `lifetime` (the old runtime's shared lease and
/// its launcher's are both gone), at most `wait`. A lock file that does not
/// exist is free.
fn wait_free(lifetime: &Path, wait: Duration) -> io::Result<()> {
    let until = Instant::now() + wait;
    loop {
        let file = match fs::OpenOptions::new().read(true).write(true).open(lifetime) {
            Ok(file) => file,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(err) => return Err(err),
        };
        if file.try_lock().is_ok() {
            // Released at once: the launcher about to start takes it.
            return Ok(());
        }
        drop(file);
        if Instant::now() >= until {
            return Err(io::Error::other("the old client did not end in time"));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// No console window for a helper or a launcher started from a window
/// (`launch::run` says why); nothing elsewhere.
fn no_window(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = command;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn install(os: Os, base: &str, program: &str) -> Install {
        Install {
            state: Some(PathBuf::from("/state/baylee/abc")),
            os,
            base: PathBuf::from(base),
            program: program.into(),
        }
    }

    #[test]
    fn the_launcher_is_the_packages_own_executable() {
        let mac = Again::launcher(
            &install(Os::MacOs, "/Applications", "Baylee.app"),
            None,
            vec![],
        );
        assert_eq!(
            mac.program,
            Path::new("/Applications/Baylee.app/Contents/MacOS/baylee-client")
        );
        assert_eq!(
            mac.lifetime.as_deref(),
            Some(Path::new("/state/baylee/abc/lifetime.lock"))
        );
        let win = Again::launcher(
            &install(Os::Windows, "C:/Baylee", "baylee-client.exe"),
            None,
            vec!["--resume".into()],
        );
        assert_eq!(win.program, Path::new("C:/Baylee/baylee-client.exe"));
        assert_eq!(win.args, vec![OsString::from("--resume")]);
        // An image that is not a file is not believed: the mount's launcher.
        let tar = Again::launcher(
            &install(Os::Linux, "/opt/baylee", "baylee-client"),
            Some(Path::new("/nowhere/Baylee.AppImage")),
            vec![],
        );
        assert_eq!(tar.program, Path::new("/opt/baylee/baylee-client"));
    }

    #[test]
    fn an_appimage_is_started_as_its_image_file() {
        let dir = std::env::temp_dir().join(format!("baylee-relaunch-{}", uuid::Uuid::now_v7()));
        fs::create_dir_all(&dir).unwrap();
        let image = dir.join("Baylee.AppImage");
        fs::write(&image, "image").unwrap();
        let again = Again::launcher(
            &install(Os::Linux, "/tmp/.mount_abc", "baylee-client"),
            Some(&image),
            vec![],
        );
        assert_eq!(again.program, image);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_helper_is_found_by_its_flag_and_nothing_else() {
        let again = Again::direct(Path::new("/x/baylee-runtime"), vec!["--resume".into()]);
        let args = vec![
            OsString::from("/x/baylee-runtime"),
            FLAG.into(),
            serde_json::to_string(&again).unwrap().into(),
        ];
        assert_eq!(asked(&args), Some(again));
        assert_eq!(asked(&[OsString::from("/x/baylee-runtime")]), None);
        assert_eq!(asked(&[OsString::from(FLAG)]), None, "no plan, no helper");
    }

    #[test]
    fn a_handoff_past_its_bound_starts_nothing() {
        let again = Again::direct(Path::new("/does/not/exist"), vec![]);
        let long = vec![b'x'; HANDOFF_BYTES + 1];
        let err = helper(&long[..], &again, Duration::ZERO).unwrap_err();
        assert!(err.to_string().contains("too long"), "{err}");
    }

    #[test]
    fn a_held_lock_is_waited_for_and_a_free_one_is_not() {
        let dir = std::env::temp_dir().join(format!("baylee-relaunch-{}", uuid::Uuid::now_v7()));
        fs::create_dir_all(&dir).unwrap();
        let lock = dir.join("lifetime.lock");
        fs::write(&lock, "").unwrap();
        let held = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lock)
            .unwrap();
        held.lock_shared().unwrap();
        let started = Instant::now();
        assert!(wait_free(&lock, Duration::from_millis(200)).is_err());
        assert!(started.elapsed() >= Duration::from_millis(200));
        drop(held);
        wait_free(&lock, Duration::ZERO).expect("free once its holder is gone");
        wait_free(&dir.join("absent.lock"), Duration::ZERO).expect("no lock, nobody holds it");
        fs::remove_dir_all(dir).unwrap();
    }
}
