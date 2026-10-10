//! The deploy-hook dispatcher (`docs/deploy-hooks.md`).
//!
//! `scripts/server/baylee-deploy` calls `sudo -n run-deploy-hooks PHASE
//! COMMIT` at three points of a server deploy. This runs every hook in
//! `/etc/baylee/deploy-hooks.d` for that phase, in byte order of the file
//! names, as root, and answers with one exit code ([`exit`]). It refuses
//! before running anything when a path on the way is not root's alone, and
//! it never follows a symbolic link: the directory and every hook are opened
//! with `O_NOFOLLOW`, checked on the open descriptor, and that descriptor is
//! what runs (`/proc/self/fd/N`), so nothing can be swapped in between the
//! check and the run.
//!
//! The installed binary reads no environment and takes no option: its paths
//! are [`Layout::production`]. A test builds another [`Layout`] in a
//! scratch directory and calls [`run::run`] directly.

pub mod args;
#[cfg(target_os = "linux")]
pub mod check;
#[cfg(target_os = "linux")]
pub mod run;
#[cfg(all(test, target_os = "linux"))]
mod scratch_tests;

use std::time::Duration;

/// The dispatcher's exit codes. Each means one thing to the deployer.
pub mod exit {
    /// Every hook ran and exited 0, or there are none.
    pub const OK: u8 = 0;
    /// A hook exited non-zero (not 75), was killed by a signal, or ran past
    /// its phase's budget. The hooks after it did not run.
    pub const FAILED: u8 = 1;
    /// The arguments were not a phase and a full commit (`EX_USAGE`).
    /// Nothing ran.
    pub const USAGE: u8 = 64;
    /// The dispatcher itself could not do its job: the log could not be
    /// opened, a hook could not be started (`EX_SOFTWARE`).
    pub const INTERNAL: u8 = 70;
    /// A hook said "not ready" by exiting 75 (`EX_TEMPFAIL`); the deploy
    /// keeps the old system and tries the same target again later. The
    /// hooks after it did not run.
    pub const NOT_READY: u8 = 75;
    /// A check refused: a directory on the way or a hook is not root's
    /// alone, is a link, is not a regular executable file, or the
    /// dispatcher is not root (`EX_NOPERM`). Nothing ran.
    pub const REFUSED: u8 = 77;
}

/// The hook name pattern, `^[A-Za-z0-9][A-Za-z0-9_-]*$`: what a file in the
/// hook directory must be called to be run. Everything else (dotfiles,
/// `name~`, `name.dpkg-old`, `name.bak`, …) is ignored.
#[must_use]
pub fn is_hook_name(name: &[u8]) -> bool {
    match name.split_first() {
        Some((first, rest)) => {
            first.is_ascii_alphanumeric()
                && rest
                    .iter()
                    .all(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'-')
        }
        None => false,
    }
}

/// The `PATH` every hook gets, and the directories whose ownership is
/// checked because a hook's commands are found in them.
pub const HOOK_PATH: &str = "/usr/sbin:/usr/bin:/sbin:/bin";

/// The locale every hook gets.
pub const HOOK_LANG: &str = "C.UTF-8";

/// Where everything is, and whose it must be.
#[derive(Clone, Debug)]
pub struct Layout {
    /// Where the walk starts; `/` on a server. It is checked too.
    pub base: std::path::PathBuf,
    /// The hook directory below `base`, one component each:
    /// `etc/baylee/deploy-hooks.d`.
    pub hooks: Vec<&'static str>,
    /// The log directory below `base`, `var/log/baylee`; the last
    /// component is made (`0750`) when it is missing.
    pub log: Vec<&'static str>,
    /// The log file in it.
    pub log_file: &'static str,
    /// The owners a checked path may have: root alone on a server.
    pub owners: Vec<u32>,
    /// Whether the dispatcher must itself run as root.
    pub require_root: bool,
    /// How long a phase may take: [`args::Phase::budget`] on a server.
    pub budget: fn(args::Phase) -> Duration,
    /// How long a hook past its budget has between `SIGTERM` and
    /// `SIGKILL` to its whole process group.
    pub grace: Duration,
}

impl Layout {
    /// The server's: `/etc/baylee/deploy-hooks.d`,
    /// `/var/log/baylee/deploy-hooks.log`, root only.
    #[must_use]
    pub fn production() -> Self {
        Self {
            base: std::path::PathBuf::from("/"),
            hooks: vec!["etc", "baylee", "deploy-hooks.d"],
            log: vec!["var", "log", "baylee"],
            log_file: "deploy-hooks.log",
            owners: vec![0],
            require_root: true,
            budget: args::Phase::budget,
            grace: Duration::from_secs(5),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_names_follow_the_pattern() {
        for good in ["10-example", "a", "Z9", "50_two-parts", "0"] {
            assert!(is_hook_name(good.as_bytes()), "{good}");
        }
        for bad in [
            "",
            ".hidden",
            "-dash",
            "_under",
            "10-example~",
            "10-example.bak",
            "10-example.dpkg-old",
            "10-example.dpkg-new",
            "with space",
            "ünï",
            ".",
            "..",
            "a/b",
        ] {
            assert!(!is_hook_name(bad.as_bytes()), "{bad}");
        }
    }

    #[test]
    fn the_server_layout_is_root_only_and_fixed() {
        let layout = Layout::production();
        assert_eq!(layout.base, std::path::Path::new("/"));
        assert_eq!(layout.hooks, ["etc", "baylee", "deploy-hooks.d"]);
        assert_eq!(layout.log, ["var", "log", "baylee"]);
        assert_eq!(layout.owners, [0]);
        assert!(layout.require_root);
    }
}
