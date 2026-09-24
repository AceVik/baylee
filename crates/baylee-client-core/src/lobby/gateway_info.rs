//! What a gateway says about itself (`GET /info`, `docs/protocol.md`
//! §"Which gateway is this? (`GET /info`)"), and what this client makes of it.
//!
//! Asked before an address is saved, so that a mistyped address is caught
//! while the player is still looking at it, and asked again of every saved
//! address, so that a gateway upgraded past this client says so in the list
//! and not at the first frame of a game.
//!
//! Everything in an answer is untrusted. The name is whatever the operator
//! typed, and a server that is not one of ours can put anything in any field.
//! So nothing is kept as it came ([`GatewayInfo::read`] cleans it on the way
//! in), and nothing here decides more than a colour and a warning: whether a
//! game opens is still decided by the view check on its first frame.

use crate::i18n::{Lang, Phrase};
use serde::Deserialize;

/// The most characters of a gateway's name this client draws.
///
/// The gateway refuses to start with a longer one (`MAX_NAME_CHARS` in
/// `baylee-gateway`), so an honest name is never cut. This is the cap for a
/// server that does not check.
pub const MAX_NAME_CHARS: usize = 64;

/// The most characters of a gateway's version line this client draws.
///
/// The real one (`baylee_build::short()`, with a commit and `-dirty`) is
/// about half of this.
pub const MAX_VERSION_CHARS: usize = 48;

/// A gateway's answer to `GET /info`, cleaned for drawing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GatewayInfo {
    /// The operator's name for it, if it has one that survives cleaning.
    pub name: Option<String>,
    /// Its build, the way a person reads it.
    pub version: String,
    /// The envelope its seat sockets speak.
    pub protocol_version: u32,
    /// The view shape its build draws.
    pub view_version: u32,
}

/// `GET /info` as it arrives.
///
/// The two versions are required: an answer without them is not the route
/// this module reads, whatever else it says, and the address is then treated
/// as one that did not answer.
#[derive(Deserialize)]
struct Wire {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    version: String,
    protocol_version: u32,
    view_version: u32,
}

impl GatewayInfo {
    /// The answer in a response body, or `None` when the body is not one.
    #[must_use]
    pub fn read(body: &[u8]) -> Option<Self> {
        let wire: Wire = serde_json::from_slice(body).ok()?;
        Some(Self {
            name: wire
                .name
                .map(|name| shown(&name, MAX_NAME_CHARS))
                .filter(|name| !name.is_empty()),
            version: shown(&wire.version, MAX_VERSION_CHARS),
            protocol_version: wire.protocol_version,
            view_version: wire.view_version,
        })
    }

    /// The version as a row has room for: the release alone, `v0.1.0`, with
    /// the build, the commit and whether it was dirty left to the hint.
    ///
    /// The release is what a player compares and the rest is what a bug
    /// report needs, so the short form cuts at the first `+` or space, which
    /// is where `baylee_build::short()` starts saying the rest. A version
    /// that is nothing but the rest, or nothing, is `v?`.
    #[must_use]
    pub fn short_version(&self) -> String {
        let release = self
            .version
            .split(|c: char| c == '+' || c.is_whitespace())
            .next()
            .unwrap_or("");
        let release = release
            .strip_prefix('v')
            .or_else(|| release.strip_prefix('V'))
            .unwrap_or(release);
        if release.is_empty() {
            return "v?".to_string();
        }
        shown(&format!("v{release}"), SHORT_VERSION_CHARS)
    }
}

/// The most characters of a short version a row draws.
pub const SHORT_VERSION_CHARS: usize = 12;

/// A string a gateway sent, as this client draws it.
///
/// Control characters would break the line, and the Unicode bidi controls
/// would reorder it into something other than what was sent, so both are
/// dropped. A string still longer than `max` characters keeps `max - 1` of
/// them and an ellipsis, so the cut is visible.
#[must_use]
pub fn shown(raw: &str, max: usize) -> String {
    let clean: String = raw
        .chars()
        .filter(|&c| !c.is_control() && !is_bidi_control(c))
        .collect();
    let clean = clean.trim();
    if clean.chars().count() <= max {
        return clean.to_string();
    }
    let mut cut: String = clean.chars().take(max.saturating_sub(1)).collect();
    cut.truncate(cut.trim_end().len());
    cut.push('…');
    cut
}

/// Unicode's embeddings, overrides and isolates: characters that are not
/// drawn and reorder the ones around them.
///
/// The set the gateway refuses in its own name (`is_bidi_control` in
/// `baylee-gateway`); the two lists are one list.
fn is_bidi_control(c: char) -> bool {
    matches!(c, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

/// What asking an address about itself came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Probe {
    /// Asked, and no answer yet.
    Asking,
    /// It answered `GET /info`.
    Known(GatewayInfo),
    /// A gateway from before `GET /info`: it answers `GET /auth/config` and
    /// says nothing about its versions.
    Older,
    /// Nothing at the address answered as a gateway. It is down, or it is
    /// some other server.
    Silent,
}

impl Probe {
    /// Whether a gateway answered, which is what saving an address needs.
    ///
    /// An incompatible gateway is still a gateway. It is saved and drawn
    /// with its warning: the player may be about to update this client, and
    /// refusing the address would only make them type it again.
    #[must_use]
    pub fn is_gateway(&self) -> bool {
        matches!(self, Self::Known(_) | Self::Older)
    }

    /// What is wrong, for a client speaking `protocol` and drawing `view`.
    #[must_use]
    pub fn warning(&self, protocol: u32, view: u32) -> Option<Warning> {
        match self {
            Self::Known(info) if info.protocol_version != protocol => Some(Warning::Protocol {
                theirs: info.protocol_version,
                ours: protocol,
            }),
            Self::Known(info) if info.view_version != view => Some(Warning::View {
                theirs: info.view_version,
                ours: view,
            }),
            Self::Known(_) | Self::Asking => None,
            Self::Older => Some(Warning::Older),
            Self::Silent => Some(Warning::Silent),
        }
    }
}

/// Why a saved gateway is drawn with a warning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Warning {
    /// Its seat sockets speak another envelope.
    Protocol {
        /// The gateway's.
        theirs: u32,
        /// This client's.
        ours: u32,
    },
    /// Its games send another view shape, which this client refuses at a
    /// game's first frame.
    View {
        /// The gateway's.
        theirs: u32,
        /// This client's.
        ours: u32,
    },
    /// It is too old to say.
    Older,
    /// It is not answering.
    Silent,
}

impl Warning {
    /// Whether its games are known not to open here, as against not known
    /// to open.
    #[must_use]
    pub fn refuses_games(self) -> bool {
        matches!(self, Self::Protocol { .. } | Self::View { .. })
    }

    /// The sentence the warning mark explains itself with.
    #[must_use]
    pub fn explain(self, lang: Lang) -> String {
        match self {
            Self::Protocol { theirs, ours } => Phrase::GatewayProtocolMismatch
                .fill(lang, &[&theirs.to_string(), &ours.to_string()]),
            Self::View { theirs, ours } => {
                Phrase::GatewayViewMismatch.fill(lang, &[&theirs.to_string(), &ours.to_string()])
            }
            Self::Older => Phrase::GatewayOlder.text(lang).to_string(),
            Self::Silent => Phrase::GatewaySilent.text(lang).to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known(protocol: u32, view: u32) -> Probe {
        Probe::Known(GatewayInfo {
            name: None,
            version: String::new(),
            protocol_version: protocol,
            view_version: view,
        })
    }

    #[test]
    fn an_answer_is_read_with_its_versions_and_refused_without_them() {
        let info = GatewayInfo::read(
            br#"{"name":"Baylee Hall","version":"0.1.0+build.7 (abc)","commit":"abc",
                "build":"7","built_at":"now","dirty":false,
                "protocol_version":3,"view_version":40}"#,
        )
        .expect("a full answer");
        assert_eq!(info.name.as_deref(), Some("Baylee Hall"));
        assert_eq!(info.version, "0.1.0+build.7 (abc)");
        assert_eq!((info.protocol_version, info.view_version), (3, 40));

        let unnamed = GatewayInfo::read(br#"{"protocol_version":1,"view_version":2}"#)
            .expect("the name and the version are optional");
        assert_eq!(unnamed.name, None);

        for not_info in [
            &br#"{"name":"x","view_version":2}"#[..],
            br#"{"protocol_version":"1","view_version":2}"#,
            b"<html>404</html>",
            b"",
        ] {
            assert_eq!(GatewayInfo::read(not_info), None);
        }
    }

    #[test]
    fn a_name_is_drawn_without_what_would_break_or_reorder_it_and_capped() {
        let read = |name: &str| {
            let body = serde_json::json!({
                "name": name, "protocol_version": 1, "view_version": 1,
            });
            GatewayInfo::read(body.to_string().as_bytes())
                .expect("an answer")
                .name
        };
        assert_eq!(
            read("  Hall\u{202E}evil\u{2066}\n  ").as_deref(),
            Some("Hallevil")
        );
        assert_eq!(read("\u{202E}\u{0007} "), None, "nothing left is no name");

        let exact = "é".repeat(MAX_NAME_CHARS);
        assert_eq!(read(&exact).as_deref(), Some(exact.as_str()));
        let long = read(&"é".repeat(MAX_NAME_CHARS + 1)).expect("a name");
        assert_eq!(long.chars().count(), MAX_NAME_CHARS);
        assert!(long.ends_with('…'));
        assert_eq!(shown("ab cd", 4), "ab…", "no space before the ellipsis");
    }

    #[test]
    fn a_short_version_is_the_release_alone() {
        let short = |version: &str| {
            GatewayInfo {
                version: version.to_string(),
                ..GatewayInfo::default()
            }
            .short_version()
        };
        assert_eq!(short("0.1.0+build.1492 (a6e3cd527c-dirty)"), "v0.1.0");
        assert_eq!(short("0.2.0 (abc)"), "v0.2.0");
        assert_eq!(short("v1.4.2"), "v1.4.2", "no second v");
        assert_eq!(short("V1.4.2+x"), "v1.4.2");
        assert_eq!(short(""), "v?");
        assert_eq!(short("+build.7"), "v?");
        let long = short("123456789012345678");
        assert_eq!(long.chars().count(), SHORT_VERSION_CHARS);
        assert!(long.ends_with('…'));
    }

    #[test]
    fn a_warning_names_the_first_version_that_differs() {
        assert_eq!(known(1, 30).warning(1, 30), None);
        assert_eq!(
            known(2, 31).warning(1, 30),
            Some(Warning::Protocol { theirs: 2, ours: 1 }),
            "a socket that cannot be read is said before a view that cannot"
        );
        assert_eq!(
            known(1, 31).warning(1, 30),
            Some(Warning::View {
                theirs: 31,
                ours: 30
            })
        );
        assert_eq!(Probe::Asking.warning(1, 30), None);
        assert_eq!(Probe::Older.warning(1, 30), Some(Warning::Older));
        assert_eq!(Probe::Silent.warning(1, 30), Some(Warning::Silent));
    }

    #[test]
    fn only_a_gateway_that_answered_is_saved_and_only_a_mismatch_refuses_games() {
        assert!(known(9, 9).is_gateway());
        assert!(Probe::Older.is_gateway());
        assert!(!Probe::Silent.is_gateway());
        assert!(!Probe::Asking.is_gateway());

        let refusing = [
            Warning::Protocol { theirs: 2, ours: 1 },
            Warning::View { theirs: 2, ours: 1 },
        ];
        assert!(refusing.iter().all(|w| w.refuses_games()));
        assert!(!Warning::Older.refuses_games());
        assert!(!Warning::Silent.refuses_games());
    }

    #[test]
    fn a_mismatch_says_both_numbers_in_every_language() {
        for lang in Lang::ALL {
            let said = Warning::View {
                theirs: 31,
                ours: 30,
            }
            .explain(lang);
            assert!(said.contains("31") && said.contains("30"), "{said}");
            let said = Warning::Protocol { theirs: 2, ours: 1 }.explain(lang);
            assert!(said.contains('2') && said.contains('1'), "{said}");
        }
    }
}
