//! A crash, written down as the client stops and sent when it next runs.
//!
//! The panic hook is the worst place in the program to do anything: the
//! state it would read may be what broke, and a network call there can hang
//! the exit. So the hook only writes a [`CrashFile`] to disk (#310), and the
//! next start reads it back and, as far as [`CrashConsent`] allows, sends it
//! as a [`Kind::Crash`] report once the player is signed in to the gateway
//! the file names ([`CrashFile::sends_to`]).
//!
//! A crash report carries the error, the build that crashed and, if the
//! player ticked it for reports, the system summary. Never the game, the
//! log, the settings or a picture: those are the form's, and the form is
//! the player's.

use serde::{Deserialize, Serialize};

use super::{BugReport, Build, Kind, Submission, System};

/// The longest panic message a crash report's `text` repeats, in characters.
const TEXT_CHARS: usize = 500;

/// What stopped the client.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrashRecord {
    /// The panic's message, the player's home directory written `~`.
    pub message: String,
    /// Where in the source it panicked (`file:line:column`).
    pub location: Option<String>,
    /// The backtrace, when one was captured, home directory written `~`.
    pub backtrace: Option<String>,
    /// When, in seconds since the Unix epoch (0: unknown).
    pub at_unix: u64,
    /// `target_os/target_arch` of the build that crashed.
    pub platform: String,
    /// The thread that panicked, by name.
    pub thread: Option<String>,
}

/// What the panic hook leaves on disk.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrashFile {
    /// The gateway the client was last signed in to when it crashed, which
    /// is where the report goes. Kept on disk only, never sent.
    pub gateway: Option<String>,
    /// The build that crashed, which is not always the one that sends it.
    pub build: Build,
    /// The crash itself.
    pub record: CrashRecord,
}

impl CrashFile {
    /// The file's contents.
    #[must_use]
    pub fn to_text(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// A file read back, or `None` for one that is not a crash file (torn by
    /// the crash it was written during, or written by something else). Such
    /// a file is deleted, not reported.
    #[must_use]
    pub fn from_text(text: &str) -> Option<Self> {
        serde_json::from_str(text).ok()
    }

    /// Whether a session at `gateway` is where this report goes: the one the
    /// client was signed in to when it crashed, or, when it was signed in
    /// nowhere, `pinned` (the live gateway every build knows).
    #[must_use]
    pub fn sends_to(&self, gateway: &str, pinned: &str) -> bool {
        let target = self.gateway.as_deref().unwrap_or(pinned);
        target.trim_end_matches('/') == gateway.trim_end_matches('/')
    }
}

/// What to do about a crash file found at start.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CrashStep {
    /// There is none.
    Nothing,
    /// Ask the player, once: send crash reports or not.
    Ask,
    /// Send it once signed in to its gateway.
    Send,
    /// Delete it unsent.
    Discard,
}

/// What `consent` says to do with a crash file, if there is one.
#[must_use]
pub fn crash_step(consent: super::CrashConsent, found: bool) -> CrashStep {
    use super::CrashConsent::{Never, Send, Unasked};
    match (found, consent) {
        (false, _) => CrashStep::Nothing,
        (true, Unasked) => CrashStep::Ask,
        (true, Send) => CrashStep::Send,
        (true, Never) => CrashStep::Discard,
    }
}

/// The report a crash file becomes. `system` is the system summary where
/// the player allows that part, else `None`.
#[must_use]
pub fn crash_submission(file: &CrashFile, system: Option<System>) -> Submission {
    let first = file.record.message.lines().next().unwrap_or_default();
    Submission {
        kind: Kind::Crash,
        text: first.chars().take(TEXT_CHARS).collect(),
        game_id: None,
        client: BugReport {
            build: file.build.clone(),
            system,
            crash: Some(file.record.clone()),
            ..BugReport::default()
        },
    }
}

/// `text` with every occurrence of `home` written `~`.
///
/// A panic message or backtrace can name a file under the player's home
/// directory, and a home directory is usually named after the person. A
/// `home` shorter than three characters (`/`, `C:`) is left alone: replacing
/// it would mangle every path without hiding anyone.
#[must_use]
pub fn scrub_home(text: &str, home: &str) -> String {
    let home = home.trim_end_matches(['/', '\\']);
    if home.len() < 3 {
        return text.to_string();
    }
    text.replace(home, "~")
}

#[cfg(test)]
mod tests {
    use super::super::CrashConsent;
    use super::*;

    fn file() -> CrashFile {
        CrashFile {
            gateway: Some("https://play.example/".into()),
            build: Build {
                version: "0.1.0-beta.1".into(),
                commit: Some("ee97ad79".into()),
            },
            record: CrashRecord {
                message: "index out of bounds: the len is 3 but the index is 7\nmore".into(),
                location: Some("crates/baylee-client/src/table.rs:12:5".into()),
                backtrace: None,
                at_unix: 1_790_000_000,
                platform: "macos/aarch64".into(),
                thread: Some("main".into()),
            },
        }
    }

    /// What the hook writes is what the next start reads.
    #[test]
    fn a_crash_file_round_trips() {
        let file = file();
        assert_eq!(CrashFile::from_text(&file.to_text()), Some(file));
    }

    /// A torn or foreign file reads as none, and is not reported.
    #[test]
    fn a_torn_file_is_not_a_crash() {
        let text = file().to_text();
        assert_eq!(CrashFile::from_text(&text[..text.len() / 2]), None);
        assert_eq!(CrashFile::from_text("hello"), None);
    }

    /// Asked once, then as answered; nothing found, nothing done.
    #[test]
    fn the_consent_decides_the_step() {
        assert_eq!(crash_step(CrashConsent::Unasked, true), CrashStep::Ask);
        assert_eq!(crash_step(CrashConsent::Send, true), CrashStep::Send);
        assert_eq!(crash_step(CrashConsent::Never, true), CrashStep::Discard);
        for consent in [
            CrashConsent::Unasked,
            CrashConsent::Send,
            CrashConsent::Never,
        ] {
            assert_eq!(crash_step(consent, false), CrashStep::Nothing);
        }
    }

    /// It goes to the gateway it crashed at, else to the pinned one.
    #[test]
    fn it_goes_to_the_gateway_it_crashed_at() {
        let pinned = "https://live.example";
        let mut file = file();
        assert!(file.sends_to("https://play.example", pinned));
        assert!(!file.sends_to(pinned, pinned));
        file.gateway = None;
        assert!(file.sends_to("https://live.example/", pinned));
    }

    /// A crash report carries the crash and the build that crashed, the
    /// system only when allowed, and no gateway address.
    #[test]
    fn a_crash_report_carries_the_crash_and_not_the_gateway() {
        let file = file();
        let submission = crash_submission(&file, None);
        assert_eq!(submission.kind, Kind::Crash);
        assert_eq!(
            submission.text,
            "index out of bounds: the len is 3 but the index is 7"
        );
        let (json, _) = submission.sealed(&[]).expect("sealed");
        assert!(json.contains("\"kind\":\"crash\""));
        assert!(json.contains("ee97ad79"));
        assert!(!json.contains("play.example"), "the gateway stays on disk");
        assert!(!json.contains("\"system\""));
    }

    /// The home directory becomes `~`; a root-sized one is left alone.
    #[test]
    fn the_home_directory_is_written_as_a_tilde() {
        assert_eq!(
            scrub_home(
                "failed to open /Users/ada/.config/baylee/x.json",
                "/Users/ada/"
            ),
            "failed to open ~/.config/baylee/x.json"
        );
        assert_eq!(
            scrub_home(r"C:\Users\ada\AppData", r"C:\Users\ada"),
            r"~\AppData"
        );
        assert_eq!(scrub_home("/tmp/x", "/"), "/tmp/x");
    }
}
