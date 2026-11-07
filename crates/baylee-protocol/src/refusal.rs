//! Why a gateway or Baylee Central refused, in a form a client understands
//! (`docs/protocol.md` §"Refusals").
//!
//! A client says a known [`Reason`] in its own words and offers its own
//! action. Everything here comes from a server that may be hostile, so what
//! reaches a player is bounded and cleaned on the way in: [`Refusal::read`]
//! is the only way a client gets one, and nothing in it is a link.
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// The most bytes of a refusal's body a client reads.
pub const MAX_REFUSAL_BODY: usize = 1024;
/// The longest `code`, in bytes.
const MAX_CODE: usize = 32;
/// The longest `min_version`, in bytes.
const MAX_VERSION: usize = 32;
/// The longest `text`, in characters.
const MAX_TEXT: usize = 200;

/// The close code of a socket closed for a refusal; its reason is the `code`.
pub const REFUSAL_CLOSE: u16 = 4000;

/// The code of a session that ended.
pub const SESSION_ENDED: &str = "session_ended";
/// The code of a banned account.
pub const BANNED: &str = "banned";

/// A refusal as it travels.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Refusal {
    /// The stable reason, `[a-z0-9_]{1,32}`.
    pub code: String,
    /// The lowest client that would be let in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_version: Option<String>,
    /// When this ends, in Unix seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<i64>,
    /// What the server adds, as plain text. See [`Refusal::server_text`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// What a client knows how to say. A code it does not know is [`Reason::Other`]
/// and still gets a plain sentence, so a new code needs no client release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// This client is too old.
    UpdateRequired,
    /// The server is closed for maintenance.
    Maintenance,
    /// The account is banned.
    Banned,
    /// Terms of use have to be accepted first.
    Terms,
    /// The session ended; signing in again may help.
    SessionEnded,
    /// The server is too busy just now.
    Busy,
    /// A code this client does not know.
    Other,
}

impl Refusal {
    /// A refusal with `code` and nothing else, as a server builds one.
    #[must_use]
    pub fn new(code: &str) -> Self {
        Self {
            code: code.to_owned(),
            ..Self::default()
        }
    }

    /// `update_required`, which a bare `426` means as well.
    #[must_use]
    pub fn update_required() -> Self {
        Self::new("update_required")
    }

    /// The reason this code stands for.
    #[must_use]
    pub fn reason(&self) -> Reason {
        match self.code.as_str() {
            "update_required" => Reason::UpdateRequired,
            "maintenance" => Reason::Maintenance,
            BANNED => Reason::Banned,
            "terms" => Reason::Terms,
            SESSION_ENDED => Reason::SessionEnded,
            "busy" => Reason::Busy,
            _ => Reason::Other,
        }
    }

    /// The refusal in an answer's body, cleaned: `None` for a body too long,
    /// not JSON, without a `refusal`, or with a `code` that breaks the rule.
    #[must_use]
    pub fn read(body: &[u8]) -> Option<Self> {
        #[derive(Deserialize)]
        struct Body {
            refusal: Refusal,
        }
        if body.len() > MAX_REFUSAL_BODY {
            return None;
        }
        serde_json::from_slice::<Body>(body).ok()?.refusal.cleaned()
    }

    /// The same rules for what arrived another way (`HelloAck`, a close
    /// frame's reason): the one door a refusal comes in by.
    #[must_use]
    pub fn cleaned(mut self) -> Option<Self> {
        if !valid_code(&self.code) {
            return None;
        }
        self.min_version = self.min_version.filter(|v| valid_version(v));
        self.text = self.text.as_deref().and_then(clean_text);
        Some(self)
    }

    /// What the server added, to be shown as its words, or nothing.
    ///
    /// Never for `update_required` or `terms`: "update required, download it
    /// from …" is what a hostile gateway would write, so their sentence and
    /// their action are this client's alone.
    #[must_use]
    pub fn server_text(&self) -> Option<&str> {
        match self.reason() {
            Reason::UpdateRequired | Reason::Terms => None,
            _ => self.text.as_deref(),
        }
    }
}

fn valid_code(code: &str) -> bool {
    !code.is_empty()
        && code.len() <= MAX_CODE
        && code
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// A version a client's own sentence can name: three numbers, and what a
/// release may add after them. Its shape is the point, so that a host name
/// cannot stand where the sentence promises a version.
fn valid_version(version: &str) -> bool {
    let shaped = |rest: &str| {
        rest.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'+' | b'-'))
    };
    let numbers = |core: &str| {
        let parts: Vec<&str> = core.split('.').collect();
        parts.len() == 3
            && parts
                .iter()
                .all(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    };
    let (core, rest) = match version.find(['-', '+']) {
        Some(at) => version.split_at(at),
        None => (version, ""),
    };
    version.len() <= MAX_VERSION && numbers(core) && shaped(rest)
}

/// Characters that are not drawn, or change how the ones around them read.
fn hidden(c: char) -> bool {
    c.is_control()
        || matches!(c,
            '\u{00AD}' | '\u{034F}' | '\u{0600}'..='\u{0605}' | '\u{061C}'
            | '\u{06DD}' | '\u{070F}' | '\u{08E2}' | '\u{115F}' | '\u{1160}'
            | '\u{17B4}' | '\u{17B5}' | '\u{180B}'..='\u{180F}'
            | '\u{200B}'..='\u{200F}' | '\u{2028}'..='\u{202E}'
            | '\u{2060}'..='\u{206F}' | '\u{2800}' | '\u{3164}'
            | '\u{FE00}'..='\u{FE0F}' | '\u{FEFF}' | '\u{FFA0}'
            | '\u{FFF0}'..='\u{FFFC}' | '\u{1D173}'..='\u{1D17A}'
            | '\u{E0000}'..='\u{E0FFF}')
}

/// Marks that could end the quotation a server's words are shown in, so that
/// what follows them would read as this client's.
const fn quote(c: char) -> bool {
    matches!(c,
        '"' | '\u{00AB}' | '\u{00BB}' | '\u{2018}'..='\u{201F}' | '\u{2039}'
        | '\u{203A}' | '\u{300C}'..='\u{300F}' | '\u{FF02}')
}

/// One line of at most [`MAX_TEXT`] drawn characters, or nothing if none stay:
/// what a server's words become before anybody reads them, whichever way they
/// came.
#[must_use]
pub fn clean_text(text: &str) -> Option<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let line: String = words
        .join(" ")
        .chars()
        .filter(|c| !hidden(*c) && !quote(*c))
        .take(MAX_TEXT)
        .collect();
    let line = line.trim();
    (!line.is_empty()).then(|| line.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(json: &str) -> Option<Refusal> {
        Refusal::read(json.as_bytes())
    }

    #[test]
    fn a_known_code_reads_with_what_the_client_needs_to_act() {
        let refusal =
            read(r#"{"refusal":{"code":"update_required","min_version":"0.1.0-beta.10"}}"#)
                .unwrap();
        assert_eq!(refusal.reason(), Reason::UpdateRequired);
        assert_eq!(refusal.min_version.as_deref(), Some("0.1.0-beta.10"));
    }

    #[test]
    fn every_known_code_has_its_reason() {
        for (code, reason) in [
            ("update_required", Reason::UpdateRequired),
            ("maintenance", Reason::Maintenance),
            ("banned", Reason::Banned),
            ("terms", Reason::Terms),
            ("session_ended", Reason::SessionEnded),
            ("busy", Reason::Busy),
        ] {
            assert_eq!(Refusal::new(code).reason(), reason, "{code}");
        }
    }

    #[test]
    fn a_new_code_is_understood_as_unknown_and_keeps_what_it_says() {
        let refusal =
            read(r#"{"refusal":{"code":"region_closed","text":"Closed here."},"more":1}"#).unwrap();
        assert_eq!(refusal.reason(), Reason::Other);
        assert_eq!(refusal.server_text(), Some("Closed here."));
    }

    #[test]
    fn what_breaks_the_rule_is_no_refusal() {
        for body in [
            "",
            "not json",
            r#"{"error":"no refusal here"}"#,
            r#"{"refusal":{"code":""}}"#,
            r#"{"refusal":{"code":"Update"}}"#,
            r#"{"refusal":{"code":"has space"}}"#,
            r#"{"refusal":{"code":"<b>x</b>"}}"#,
            r#"{"refusal":{"code":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}}"#,
        ] {
            assert_eq!(read(body), None, "{body}");
        }
    }

    #[test]
    fn a_body_past_the_bound_is_not_read() {
        let body = format!(
            r#"{{"refusal":{{"code":"busy"}},"pad":"{}"}}"#,
            "x".repeat(MAX_REFUSAL_BODY)
        );
        assert_eq!(read(&body), None);
    }

    #[test]
    fn a_version_cannot_carry_words() {
        for version in [
            "go to evil.example",
            "https://evil.example",
            "1.0 update here",
            "",
            "000000000000000000000000000000000",
            "baylee-update.evil.example",
            "evil.example",
            "1.2",
            "1.2.3.evil.example",
        ] {
            let refusal = Refusal {
                code: "update_required".into(),
                min_version: Some(version.into()),
                ..Refusal::default()
            };
            assert_eq!(refusal.cleaned().unwrap().min_version, None, "{version}");
        }
    }

    #[test]
    fn a_release_is_a_version() {
        for version in ["0.1.0", "0.1.0-beta.10", "0.1.0-beta.9c", "1.2.3+build.4"] {
            let refusal = Refusal {
                code: "update_required".into(),
                min_version: Some(version.into()),
                ..Refusal::default()
            };
            let cleaned = refusal.cleaned().unwrap();
            assert_eq!(cleaned.min_version.as_deref(), Some(version));
        }
    }

    /// Nothing the eye cannot see stays, in any of the blocks that hold such
    /// characters.
    #[test]
    fn what_is_not_drawn_does_not_stay() {
        for c in [
            '\u{E0041}', // a tag
            '\u{E0100}', // a variation selector
            '\u{180B}',  // Mongolian
            '\u{3164}',  // a Hangul filler
            '\u{2800}',  // a blank Braille cell
            '\u{1D173}', // a musical format mark
            '\u{0600}',  // an Arabic format mark
            '\u{FFFC}',  // an object replacement
        ] {
            let text = format!("a{c}b");
            assert_eq!(clean_text(&text).as_deref(), Some("ab"), "{c:?}");
        }
    }

    /// A server's words are shown in quotation marks, so they may not hold
    /// the mark that ends them and go on in this client's voice.
    #[test]
    fn a_server_cannot_close_the_quotation_it_is_shown_in() {
        for text in ["busy” Update now", "busy\" Update now", "busy» Update now"] {
            assert_eq!(clean_text(text).as_deref(), Some("busy Update now"));
        }
    }

    #[test]
    fn the_two_codes_a_phish_would_use_never_show_server_text() {
        for code in ["update_required", "terms"] {
            let refusal = Refusal {
                code: code.into(),
                text: Some("Download the update from evil.example".into()),
                ..Refusal::default()
            };
            assert_eq!(refusal.cleaned().unwrap().server_text(), None, "{code}");
        }
    }

    #[test]
    fn text_is_one_short_plain_line() {
        let refusal = |text: &str| Refusal {
            code: "maintenance".into(),
            text: Some(text.into()),
            ..Refusal::default()
        };
        let long = refusal(&"x".repeat(MAX_TEXT * 2)).cleaned().unwrap();
        assert_eq!(long.text.unwrap().chars().count(), MAX_TEXT);
        let lines = refusal("back\n\n  at\tnine\r\n").cleaned().unwrap();
        assert_eq!(lines.text.as_deref(), Some("back at nine"));
    }

    #[test]
    fn what_changes_how_text_reads_is_removed() {
        let refusal = Refusal {
            code: "maintenance".into(),
            text: Some("pay\u{202E}moc.live\u{202C}\u{200B}\u{FEFF}\u{0007} now".into()),
            ..Refusal::default()
        };
        assert_eq!(
            refusal.cleaned().unwrap().text.as_deref(),
            Some("paymoc.live now")
        );
    }

    #[test]
    fn text_of_nothing_visible_is_no_text() {
        let refusal = Refusal {
            code: "busy".into(),
            text: Some("\u{200B}\u{202E} \n".into()),
            ..Refusal::default()
        };
        assert_eq!(refusal.cleaned().unwrap().text, None);
    }

    #[test]
    fn it_travels_with_only_the_fields_it_has() {
        let json = serde_json::to_string(&Refusal::update_required()).unwrap();
        assert_eq!(json, r#"{"code":"update_required"}"#);
    }
}
