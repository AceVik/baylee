//! End-to-end tests of the desktop client (`docs/dev/e2e.md`).
//!
//! Every test starts a real `baylee-client` built with `dev-control`, drives
//! it through the loopback harness the way a player would (pointer, wheel,
//! keys, typed text, window size) and reads back what it shows: the interface
//! tree with its texts and rectangles, the controls with their `Press` names,
//! and anything else `/state` reports. A test reads like what it checks:
//!
//! ```no_run
//! use baylee_client_e2e::prelude::*;
//!
//! #[test]
//! #[ignore = "opens a client window: cargo dev e2e"]
//! fn playing_offline_offers_the_house() {
//!     e2e::run(|client| {
//!         client.click(press("Front(PlayOffline)"))?;
//!         let screen = client.wait_until("the offline lobby", |s| s.flag("/account/offline"))?;
//!         screen.expect_shows("Play the house")?;
//!         Ok(())
//!     });
//! }
//! ```
//!
//! A window size is a [`Viewport`]; [`Client::across`] runs one check at
//! several and reports every size that failed, and [`Screen::layout`] finds
//! what a resize breaks (a control out of the window, text wider than its
//! box, two controls on top of each other, a target too small to touch).
//!
//! The tests open windows, so they are `#[ignore]`d: the gates compile them
//! and run this crate's own unit tests, and `cargo dev e2e` builds the client
//! once and runs them. A failed test leaves a screenshot, the screen's
//! reading and the client's logs under `target/e2e/<run>/<test>/`.
// Every fallible call here fails a test, and fails it with a message that
// says what it waited for or could not find; an `# Errors` section on each
// would say "when the client does not do it" fifty times.
#![allow(clippy::missing_errors_doc)]

pub mod client;
pub mod devctl;
pub mod flows;
pub mod layout;
pub mod monitor;
pub mod screen;
pub mod viewport;

pub use client::{Client, Key, Launch};
pub use devctl::{Devctl, Refused};
pub use layout::{Issue, Layout};
pub use monitor::Monitor;
pub use screen::{Card, Clip, Control, ControlList, Node, Rect, Screen, Target};
pub use viewport::{TextStep, Viewport};

use anyhow::Result;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
    sync::{Condvar, Mutex},
};

/// Everything a test file needs, in one `use`.
pub mod prelude {
    pub use crate::screen::{Card, at, card, in_hand, label, press, press_prefix, text};
    pub use crate::{
        Client, Issue, Key, Launch, Layout, Monitor, Rect, Screen, Target, TextStep, Viewport, e2e,
    };
    pub use anyhow::{Result, bail, ensure};
}

/// The entry points a test calls (`e2e::run`).
pub mod e2e {
    pub use crate::{run, run_with};
}

/// Runs a test against a client started with the defaults
/// ([`Launch::new`]): see [`run_with`].
pub fn run(test: impl FnOnce(&mut Client) -> Result<()>) {
    run_with(Launch::new(), test);
}

/// Starts a client, runs the test against it and closes it, whatever the
/// test did. A test that returns an error or panics leaves a screenshot and
/// the screen's reading beside the client's logs, and fails naming where.
///
/// # Panics
///
/// When the client cannot be started, the test fails, or the client cannot
/// be shown closed afterwards.
#[allow(clippy::needless_pass_by_value)] // `run_with(Launch::new().lang("de"), …)` reads as a sentence
pub fn run_with(launch: Launch, test: impl FnOnce(&mut Client) -> Result<()>) {
    let name = test_name();
    let dir = artifacts().join(name.replace("::", "/"));
    let _ = std::fs::remove_dir_all(&dir);
    if let Err(error) = std::fs::create_dir_all(&dir) {
        panic!("{name}: cannot make {}: {error}", dir.display());
    }
    let _slot = Slot::take();
    let mut client = match Client::start(&client_binary(), &launch, &dir) {
        Ok(client) => client,
        Err(error) => panic!(
            "{name}: the client did not start: {error:#}\n  logs: {}",
            dir.display()
        ),
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| test(&mut client)));
    let failure = match outcome {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(format!("{error:#}")),
        Err(panic) => Some(
            panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                .unwrap_or_else(|| "the test panicked".to_owned()),
        ),
    };
    if failure.is_some() {
        if let Ok(screen) = client.screen() {
            client.keep("failure", &screen);
        }
        let _ = client.screenshot("failure");
    }
    let closed = client.close();
    if let Some(failure) = failure {
        panic!("{name} failed: {failure}\n  artifacts: {}", dir.display());
    }
    if let Err(error) = closed {
        panic!("{name}: the client was not shown closed: {error:#}");
    }
}

/// The test's name as libtest gives its thread (`front_door::fits`).
fn test_name() -> String {
    std::thread::current()
        .name()
        .filter(|name| *name != "main")
        .unwrap_or("e2e")
        .to_owned()
}

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `target/e2e/<run>`: `BAYLEE_E2E_RUN` names the run (`cargo dev e2e` sets
/// it), else `local`.
fn artifacts() -> PathBuf {
    let run = std::env::var("BAYLEE_E2E_RUN").unwrap_or_else(|_| "local".to_owned());
    workspace().join("target/e2e").join(run)
}

/// The client under test: `BAYLEE_E2E_CLIENT` (`cargo dev e2e` sets it to a
/// copy no other build can replace mid-run), else this checkout's debug build.
fn client_binary() -> PathBuf {
    std::env::var_os("BAYLEE_E2E_CLIENT").map_or_else(
        || {
            workspace()
                .join("target/debug/baylee-client")
                .with_extension(std::env::consts::EXE_EXTENSION)
        },
        PathBuf::from,
    )
}

/// How many clients may be open at once in this process
/// (`BAYLEE_E2E_JOBS`, 1 by default): every client is a window and a GPU
/// context, and tests that time a client out under load prove nothing.
struct Slot;

static OPEN: Mutex<usize> = Mutex::new(0);
static FREED: Condvar = Condvar::new();

impl Slot {
    fn take() -> Self {
        let limit = std::env::var("BAYLEE_E2E_JOBS")
            .ok()
            .and_then(|jobs| jobs.parse().ok())
            .unwrap_or(1_usize)
            .max(1);
        let mut open = OPEN
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        while *open >= limit {
            open = FREED
                .wait(open)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        *open += 1;
        Self
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        let mut open = OPEN
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *open -= 1;
        FREED.notify_one();
    }
}

#[cfg(test)]
mod tests;
