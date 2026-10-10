//! Running one phase's hooks: one at a time, each in its own process group,
//! under the phase's budget, with its output in the root-only log.
//!
//! A hook is the file [`crate::check`] opened, run as `/proc/self/fd/N`:
//! the descriptor is made inheritable for that one spawn, so a script's
//! interpreter reopens the very file that was checked. The hook gets
//! `PATH` and `LANG` and nothing else from the environment, `/` as its
//! working directory, `/dev/null` as standard input, and the log as standard
//! output and error. When it exits, whatever it left running in its process
//! group is killed before it is reaped (the zombie keeps the group's id from
//! being reused); past the budget the group gets `SIGTERM`, then `SIGKILL`
//! after the grace.

use std::fmt;
use std::fs::File;
use std::io::Write as _;
use std::os::fd::AsRawFd as _;
use std::os::unix::process::{CommandExt as _, ExitStatusExt as _};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use rustix::fs::{FileType, Mode, OFlags};
use rustix::io::{Errno, FdFlags};
use rustix::process::{Pid, Signal, WaitId, WaitIdOptions};

use crate::args::Phase;
use crate::check::{self, Hook, Refusal};
use crate::{HOOK_LANG, HOOK_PATH, Layout, exit};

/// How often a running hook is looked at.
const POLL: Duration = Duration::from_millis(25);

/// How a hook ended, when it did not exit 0 or 75.
#[derive(Debug, PartialEq, Eq)]
pub enum Ended {
    /// It exited with this status.
    Exit(i32),
    /// A signal killed it.
    Signal(i32),
    /// It ran past the phase's budget and its group was killed.
    TimedOut(Duration),
}

/// What a phase came to. [`Outcome::code`] is the exit code; the
/// [`fmt::Display`] is neutral (a hook's place, never its name).
#[derive(Debug)]
pub enum Outcome {
    /// No hook directory, or nothing in it that matches the name pattern.
    NoHooks,
    /// This many hooks ran, and all exited 0.
    Ran(usize),
    /// Hook `index` of `count` exited 75.
    NotReady {
        /// 1-based.
        index: usize,
        /// Of how many.
        count: usize,
    },
    /// Hook `index` of `count` failed.
    Failed {
        /// 1-based.
        index: usize,
        /// Of how many.
        count: usize,
        /// How.
        how: Ended,
    },
    /// A check refused; nothing ran.
    Refused(Refusal),
    /// The dispatcher could not do its job.
    Internal(String),
}

impl Outcome {
    /// The dispatcher's exit code for this outcome ([`crate::exit`]).
    #[must_use]
    pub fn code(&self) -> u8 {
        match self {
            Self::NoHooks | Self::Ran(_) => exit::OK,
            Self::NotReady { .. } => exit::NOT_READY,
            Self::Failed { .. } => exit::FAILED,
            Self::Refused(_) => exit::REFUSED,
            Self::Internal(_) => exit::INTERNAL,
        }
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHooks => f.write_str("no hooks"),
            Self::Ran(count) => write!(f, "{count} hook(s) ran"),
            Self::NotReady { index, count } => {
                write!(f, "hook {index} of {count} is not ready (exit 75)")
            }
            Self::Failed { index, count, how } => match how {
                Ended::Exit(code) => write!(f, "hook {index} of {count} exited with status {code}"),
                Ended::Signal(signal) => {
                    write!(f, "hook {index} of {count} was killed by signal {signal}")
                }
                Ended::TimedOut(budget) => write!(
                    f,
                    "hook {index} of {count} ran past the phase's {}s and was stopped",
                    budget.as_secs()
                ),
            },
            Self::Refused(why) => write!(f, "refused: {why}"),
            Self::Internal(why) => write!(f, "could not run: {why}"),
        }
    }
}

/// Runs `phase`'s hooks for `commit` under `layout`.
#[must_use]
pub fn run(layout: &Layout, phase: Phase, commit: &str) -> Outcome {
    if layout.require_root && !rustix::process::geteuid().is_root() {
        return Outcome::Refused(Refusal::NotRoot);
    }
    // Fixed, so a hook does not inherit whatever the caller had.
    rustix::process::umask(Mode::from_raw_mode(0o022));
    let shown = layout
        .hooks
        .iter()
        .fold(layout.base.clone(), |p, c| p.join(c));
    let dir = match check::open_chain(&layout.base, &layout.hooks, &layout.owners) {
        Ok(Some(dir)) => dir,
        Ok(None) => return Outcome::NoHooks,
        Err(why) => return Outcome::Refused(why),
    };
    let names = match check::hook_names(&dir, &shown) {
        Ok(names) if names.is_empty() => return Outcome::NoHooks,
        Ok(names) => names,
        Err(why) => return Outcome::Refused(why),
    };
    let mut log = match open_log(layout) {
        Ok(log) => log,
        Err(why) => return Outcome::Internal(why),
    };
    let checked = check::search_path(&layout.owners)
        .and_then(|()| check::open_hooks(&dir, &names, &layout.owners));
    let hooks = match checked {
        Ok(hooks) => hooks,
        Err(why) => {
            note(
                &mut log,
                &format!("{phase} {commit}: refused: {}", why.detail()),
            );
            return Outcome::Refused(why);
        }
    };
    let outcome = run_all(layout, &hooks, phase, commit, &mut log);
    note(&mut log, &format!("{phase} {commit}: {outcome}"));
    outcome
}

/// Each hook in turn until one does not exit 0.
fn run_all(layout: &Layout, hooks: &[Hook], phase: Phase, commit: &str, log: &mut File) -> Outcome {
    let budget = (layout.budget)(phase);
    let deadline = Instant::now() + budget;
    let count = hooks.len();
    for hook in hooks {
        let index = hook.index;
        note(
            log,
            &format!(
                "{phase} {commit}: hook {index} of {count} ({}) starts",
                hook.name
            ),
        );
        let mut child = match spawn(hook, phase, commit, log) {
            Ok(child) => child,
            Err(e) => {
                note(
                    log,
                    &format!("hook {index} ({}) could not be started: {e}", hook.name),
                );
                return Outcome::Internal(format!("hook {index} of {count} could not be started"));
            }
        };
        let ended = match wait(&mut child, deadline, budget, layout.grace) {
            Ok(ended) => ended,
            Err(e) => {
                return Outcome::Internal(format!("waiting for hook {index} of {count}: {e}"));
            }
        };
        match ended {
            Ended::Exit(0) => {}
            Ended::Exit(75) => return Outcome::NotReady { index, count },
            how => return Outcome::Failed { index, count, how },
        }
    }
    Outcome::Ran(count)
}

/// Starts one hook from its checked descriptor.
fn spawn(hook: &Hook, phase: Phase, commit: &str, log: &File) -> std::io::Result<Child> {
    // Inheritable for this one spawn: a script's interpreter opens
    // `/proc/self/fd/N` in the new process, which is this same file.
    rustix::io::fcntl_setfd(&hook.fd, FdFlags::empty())?;
    let started = Command::new(format!("/proc/self/fd/{}", hook.fd.as_raw_fd()))
        .arg0(&hook.name)
        .args([phase.word(), commit])
        .env_clear()
        .env("PATH", HOOK_PATH)
        .env("LANG", HOOK_LANG)
        .current_dir("/")
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log.try_clone()?)
        .process_group(0)
        .spawn();
    rustix::io::fcntl_setfd(&hook.fd, FdFlags::CLOEXEC)?;
    started
}

/// Whether the child has exited, without reaping it.
fn exited(pid: Pid) -> std::io::Result<bool> {
    loop {
        match rustix::process::waitid(
            WaitId::Pid(pid),
            WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
        ) {
            Ok(status) => return Ok(status.is_some()),
            Err(Errno::INTR) => {}
            Err(e) => return Err(e.into()),
        }
    }
}

/// Waits for the hook until `deadline`; kills its whole group on the way
/// out either way.
fn wait(
    child: &mut Child,
    deadline: Instant,
    budget: Duration,
    grace: Duration,
) -> std::io::Result<Ended> {
    let pid = Pid::from_child(child);
    while !exited(pid)? {
        if Instant::now() >= deadline {
            let _ = rustix::process::kill_process_group(pid, Signal::TERM);
            let until = Instant::now() + grace;
            while Instant::now() < until && !exited(pid)? {
                std::thread::sleep(POLL);
            }
            let _ = rustix::process::kill_process_group(pid, Signal::KILL);
            child.wait()?;
            return Ok(Ended::TimedOut(budget));
        }
        std::thread::sleep(POLL);
    }
    // The leader is a zombie now and holds the group's id, so this reaches
    // exactly what it left behind and nothing else.
    let _ = rustix::process::kill_process_group(pid, Signal::KILL);
    let status = child.wait()?;
    Ok(match (status.code(), status.signal()) {
        (Some(code), _) => Ended::Exit(code),
        (None, Some(signal)) => Ended::Signal(signal),
        (None, None) => Ended::Signal(0),
    })
}

/// Opens (and makes, `0750`) the log directory and appends to the log,
/// `0600`. The directory must be a listed owner's and not group/other
/// writable, the file a regular one of a listed owner; neither may be a
/// link.
fn open_log(layout: &Layout) -> Result<File, String> {
    let (last, parents) = layout.log.split_last().ok_or("no log directory")?;
    let parent = parents.iter().fold(layout.base.clone(), |p, c| p.join(c));
    let shown = parent.join(last);
    let dir_flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    // The parents are not judged: `/var/log` is group-writable on common
    // systems. The directory itself is, on its descriptor.
    let parent_fd = rustix::fs::open(&parent, dir_flags, Mode::empty())
        .map_err(|e| format!("{} cannot be opened: {e}", parent.display()))?;
    let dir = match rustix::fs::openat(&parent_fd, *last, dir_flags, Mode::empty()) {
        Err(Errno::NOENT) => {
            rustix::fs::mkdirat(&parent_fd, *last, Mode::from_raw_mode(0o750))
                .map_err(|e| format!("{} cannot be made: {e}", shown.display()))?;
            rustix::fs::openat(&parent_fd, *last, dir_flags, Mode::empty())
        }
        other => other,
    }
    .map_err(|e| format!("{} cannot be opened: {e}", shown.display()))?;
    let stat = rustix::fs::fstat(&dir).map_err(|e| e.to_string())?;
    if !layout.owners.contains(&stat.st_uid) || stat.st_mode & 0o022 != 0 {
        return Err(format!("{} is not root's alone", shown.display()));
    }
    let file = rustix::fs::openat(
        &dir,
        layout.log_file,
        OFlags::WRONLY | OFlags::APPEND | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )
    .map_err(|e| format!("the log in {} cannot be opened: {e}", shown.display()))?;
    let stat = rustix::fs::fstat(&file).map_err(|e| e.to_string())?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
        || !layout.owners.contains(&stat.st_uid)
    {
        return Err(format!(
            "the log in {} is not a file of root's",
            shown.display()
        ));
    }
    if stat.st_mode & 0o777 != 0o600 {
        rustix::fs::fchmod(&file, Mode::from_raw_mode(0o600)).map_err(|e| e.to_string())?;
    }
    Ok(File::from(file))
}

/// One line in the root-only log, stamped with the Unix time. A log that
/// cannot be written to does not stop the deploy.
fn note(log: &mut File, line: &str) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let _ = writeln!(log, "== {now} run-deploy-hooks: {line}");
}
