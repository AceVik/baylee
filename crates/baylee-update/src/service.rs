//! The updater's own thread: when to check, and nothing else.
//!
//! The client starts one [`Service`] and reads its answers from a channel,
//! so a check (a request, a 150 MB download, a signature over all of it,
//! the unpacking) never runs on the client's frame. It checks once at start
//! and then every [`EVERY`], and whenever the player presses "Check for
//! updates" ([`Command::CheckNow`]).
//!
//! With automatic checks off ([`Settings::check`]) it makes **no request at
//! all** until the player asks for one: not at start, not every six hours.
//! That is the privacy promise `docs/privacy.md` makes, and
//! `tests/end_to_end.rs` counts the requests a stub server receives to hold
//! it.

use crate::check::{Checker, Context, Manual, Outcome};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

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

/// The running thread's door.
pub struct Service {
    commands: Sender<Command>,
    /// Every check's outcome, in order.
    pub outcomes: Receiver<Outcome>,
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
        let (answers, outcomes) = mpsc::channel();
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
        Ok(Self { commands, outcomes })
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
    answers: &Sender<Outcome>,
) {
    let mut due = settings.check;
    loop {
        if due {
            context.installs = installs(settings, dev);
            if answers.send(checker.run(&context)).is_err() {
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
