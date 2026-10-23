//! The updater's own thread: when to check, and what it says while it does.
//!
//! The client starts one [`Service`] and reads its [`Report`]s from a
//! channel, so a check (a request, a 150 MB download, a signature over all
//! of it, the unpacking) never runs on the client's frame. It checks once at
//! start and then every [`EVERY`], and whenever the player presses "Check
//! for updates" ([`Command::CheckNow`]). Every check begins with
//! [`Report::Checking`], says how far staging a newer release has got
//! ([`Report::Stage`], a few times a second while it downloads), and ends
//! with [`Report::Done`].
//!
//! With automatic checks off ([`Settings::check`]) it makes **no request at
//! all** until the player asks for one: not at start, not every six hours.
//! That is the privacy promise `docs/privacy.md` makes, and
//! `tests/end_to_end.rs` counts the requests a stub server receives to hold
//! it.

use crate::check::{Checker, Context, Manual, Outcome};
use crate::progress::Stage;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

/// How often a running client checks.
pub const EVERY: Duration = Duration::from_hours(6);

/// What the player chose, per device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// Ask GitHub at start and every six hours. Off: never, unless asked.
    pub check: bool,
    /// Download and install a newer release by itself. Off: the notice
    /// only links to it.
    pub install: bool,
}

/// What the client can tell the thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// Check now, whatever [`Settings::check`] says: the player asked.
    CheckNow,
    /// The player changed a setting.
    Settings(Settings),
}

/// What the thread says, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Report {
    /// A check begins: its request for the releases leaves now (unless
    /// GitHub's limit is still spent). `at` is when, so a client can say
    /// how soon after its own start it asked.
    Checking {
        /// When the request was made.
        at: Instant,
    },
    /// How far staging a newer release has got.
    Stage(Stage),
    /// The check's end.
    Done(Outcome),
}

impl Report {
    /// The check's end, if this is it.
    #[must_use]
    pub fn outcome(self) -> Option<Outcome> {
        match self {
            Self::Done(outcome) => Some(outcome),
            Self::Checking { .. } | Self::Stage(_) => None,
        }
    }
}

/// The running thread's door.
pub struct Service {
    commands: Sender<Command>,
    /// Everything the thread says, in order.
    pub reports: Receiver<Report>,
}

impl Service {
    /// Starts the thread. `dev` is a development build, which checks and
    /// never installs.
    ///
    /// # Errors
    ///
    /// When the system refuses a thread.
    pub fn start(
        api: String,
        context: Context,
        settings: Settings,
        dev: bool,
        every: Duration,
    ) -> std::io::Result<Self> {
        let (commands, inbox) = mpsc::channel();
        let (answers, reports) = mpsc::channel();
        std::thread::Builder::new()
            .name("baylee-update".into())
            .spawn(move || {
                let mut checker = Checker::new(api, &context.current.to_string());
                run(
                    &mut checker,
                    context,
                    settings,
                    dev,
                    every,
                    &inbox,
                    &answers,
                );
            })?;
        Ok(Self { commands, reports })
    }

    /// Tells the thread something. Ignored once it has stopped.
    pub fn send(&self, command: Command) {
        let _ = self.commands.send(command);
    }
}

fn installs(settings: Settings, dev: bool) -> Result<(), Manual> {
    if dev {
        Err(Manual::DevBuild)
    } else if settings.install {
        Ok(())
    } else {
        Err(Manual::Off)
    }
}

fn run(
    checker: &mut Checker,
    mut context: Context,
    mut settings: Settings,
    dev: bool,
    every: Duration,
    inbox: &Receiver<Command>,
    answers: &Sender<Report>,
) {
    let mut due = settings.check;
    loop {
        if due {
            context.installs = installs(settings, dev);
            if answers
                .send(Report::Checking { at: Instant::now() })
                .is_err()
            {
                return;
            }
            // A client that has gone stops a download in the middle.
            let outcome = checker.run_reporting(&context, &mut |stage| {
                answers.send(Report::Stage(stage)).is_ok()
            });
            if answers.send(Report::Done(outcome)).is_err() {
                return;
            }
        }
        due = match inbox.recv_timeout(every) {
            Ok(Command::CheckNow) => true,
            Ok(Command::Settings(next)) => {
                settings = next;
                false
            }
            Err(RecvTimeoutError::Timeout) => settings.check,
            Err(RecvTimeoutError::Disconnected) => return,
        };
    }
}
