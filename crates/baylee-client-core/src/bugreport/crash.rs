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

/// The longest backtrace a crash report carries, in characters.
///
/// A debug build's backtrace of a panic deep in Bevy's schedule runs to a
/// hundred frames and some 30 KB; a release build's is a few KB. The frames
/// that say what broke are the first ones, so the cap keeps the head and
/// says how much was cut. Far inside [`super::MAX_CLIENT_BYTES`], so it never
/// pushes a crash report over the gateway's limit.
pub const BACKTRACE_CHARS: usize = 16_000;

/// A captured backtrace as a crash report carries it: the home directory
/// written `~` (a frame names the file it was compiled from, and a
/// dependency's lies under `~/.cargo`), then cut to [`BACKTRACE_CHARS`] at
/// a line boundary, with a last line saying how many lines were dropped.
/// `None` for one that captured nothing (a platform without unwinding
/// information says only "unsupported" or "disabled backtrace").
#[must_use]
pub fn bounded_backtrace(text: &str, home: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() || matches!(trimmed, "unsupported backtrace" | "disabled backtrace") {
        return None;
    }
    let scrubbed = scrub_home(text.trim_end(), home);
    if scrubbed.chars().count() <= BACKTRACE_CHARS {
        return Some(scrubbed);
    }
    let lines: Vec<&str> = scrubbed.lines().collect();
    let mut kept = String::new();
    let mut used = 0;
    let mut taken = 0;
    // Room for the closing line, which is at most this long.
    let budget = BACKTRACE_CHARS - 40;
    for line in &lines {
        let length = line.chars().count() + 1;
        if used + length > budget {
            break;
        }
        kept.push_str(line);
        kept.push('\n');
        used += length;
        taken += 1;
    }
    if taken == 0 {
        // One line longer than the whole budget: cut inside it.
        kept = scrubbed.chars().take(budget).collect();
        kept.push('\n');
        taken = 1;
    }
    kept.push_str(&format!("… {} more lines", lines.len() - taken));
    Some(kept)
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

    /// A backtrace comes with the home directory written `~`, whole when it
    /// is short, and cut to its head at a line boundary when it is long.
    #[test]
    fn a_backtrace_is_scrubbed_and_bounded_from_its_head() {
        let home = "/Users/ada";
        let short = "   0: std::panicking::begin_panic\n             at /Users/ada/.cargo/registry/src/x.rs:1:2";
        let kept = bounded_backtrace(short, home).expect("a backtrace");
        assert!(kept.contains("~/.cargo/registry") && !kept.contains("/Users/ada"));
        assert!(!kept.contains("more lines"), "a short one is whole");

        let frames: Vec<String> = (0..2_000)
            .map(|i| format!("  {i:4}: baylee_client::frame_{i}\n             at /Users/ada/src/f.rs:{i}:1"))
            .collect();
        let long = frames.join("\n");
        assert!(long.chars().count() > 4 * BACKTRACE_CHARS);
        let kept = bounded_backtrace(&long, home).expect("a backtrace");
        assert!(kept.chars().count() <= BACKTRACE_CHARS, "{}", kept.chars().count());
        assert!(kept.starts_with("     0: baylee_client::frame_0"), "the head is kept");
        assert!(!kept.contains("frame_1999"), "the tail is what goes");
        assert!(!kept.contains("/Users/ada"));
        let last = kept.lines().last().expect("a last line");
        let dropped: usize = last
            .trim_start_matches("… ")
            .trim_end_matches(" more lines")
            .parse()
            .expect("the last line counts what was cut");
        assert_eq!(kept.lines().count() - 1 + dropped, long.lines().count());

        let one_line = "x".repeat(3 * BACKTRACE_CHARS);
        let kept = bounded_backtrace(&one_line, home).expect("a backtrace");
        assert!(kept.chars().count() <= BACKTRACE_CHARS);
    }

    /// A platform that captured nothing sends no backtrace, rather than a
    /// field that says so.
    #[test]
    fn an_empty_backtrace_is_none() {
        for nothing in ["", "  ", "unsupported backtrace", "disabled backtrace"] {
            assert_eq!(bounded_backtrace(nothing, "/Users/ada"), None, "{nothing:?}");
        }
    }

    /// A crash report carries its backtrace, and stays well inside the
    /// gateway's limit with the longest one it can have.
    #[test]
    fn a_crash_report_carries_its_backtrace() {
        let mut file = file();
        file.record.backtrace = bounded_backtrace(&"   0: frame\n".repeat(50_000), "/Users/ada");
        let (json, _) = crash_submission(&file, None).sealed(&[]).expect("sealed");
        assert!(json.contains("\"backtrace\":\"   0: frame"));
        assert!(json.len() < super::super::MAX_CLIENT_BYTES / 10);
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
