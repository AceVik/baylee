//! What a bug report carries, and what it must never carry.
//!
//! A player who has just watched the game do the wrong thing is holding the
//! only copy of the evidence, and the window that says "what went wrong?" is
//! the one chance to collect it. So this reaches for everything it can — the
//! whole board as the seat was shown it, the question the engine was asking,
//! what the client had selected and armed, the last refusal it was given —
//! and the free text beside it, because the one thing no dump contains is
//! what the player *expected* to happen.
//!
//! # Why that is safe
//!
//! The alarming half of "send me everything" is that a client is a program a
//! player runs. Two rules, and they are different rules with different
//! reasons.
//!
//! **An opponent's information cannot be in here, because the client never
//! had it.** `baylee-view` has no field to leak through: a library is a
//! count, another seat's hand is a count, a face-down permanent's `card` is
//! `None` for anyone not entitled to look. That is the platform's own rule
//! (`docs/protocol.md`, "hidden information is unrepresentable, not
//! omitted"), and it is what makes a report of the *whole* `PlayerView`
//! safe to attach without reading it first. Nothing has to be filtered out,
//! because nothing was ever put in.
//!
//! **The reporter's own secrets are kept out by an allow-list, never by a
//! filter.** [`BugReport`] is a named struct assembled one field at a time;
//! there is deliberately no "and the rest of the client state" field, and
//! the `dev-control` `/state` dump — which is the tempting shortcut — is
//! exactly the wrong shape, because it is a debugging surface that grows,
//! and the day it grows a field holding a token, a dump ships the token and
//! a struct does not. A seat token, a session token and a gateway URL
//! carrying either are not fields here, and neither is the account's
//! address: a gateway learns *who* from the request it authenticated, so
//! identity never has to ride in the payload at all.
//!
//! And because "we were careful" is not a property anything can check,
//! [`seal`] makes it one. The sender hands over the secrets it is holding
//! and gets the bytes back only if none of them appears in the report. It
//! fails closed, at run time, in the shipped build — so a field added later
//! that happens to carry a token is a refused report rather than a leaked
//! one.

use serde::Serialize;

use baylee_engine::choice::Pending;
use baylee_view::PlayerView;

/// The shortest string [`seal`] will search for.
///
/// A secret short enough to occur by accident would make every report a
/// refused one: a two-character token appears in the first card name that
/// contains it. Anything this platform calls a token is 32 hex characters or
/// a base64url blob, so the bound costs nothing real — and a "secret" below
/// it is not one, whatever it is stored in.
pub const SHORTEST_SECRET: usize = 12;

/// One thing the sender is holding that must not appear in a report.
///
/// The `label` is what a refusal is allowed to say. The value never is: an
/// error message naming the string it found would be a way to read a token
/// out of a log.
#[derive(Clone, Copy, Debug)]
pub struct Secret<'a> {
    /// What it is, for a refusal to name.
    pub label: &'a str,
    /// The bytes that must not be in the report.
    pub value: &'a str,
}

/// A report that was refused because it contained something it must not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Leaked {
    /// Which secret, by its label. Never the secret itself.
    pub label: String,
}

impl std::fmt::Display for Leaked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the report contains the {}; refusing to send it",
            self.label
        )
    }
}

impl std::error::Error for Leaked {}

/// What the player said.
///
/// Two fields and not one, because they are two different things and the
/// second is the one a dump can never supply. "It did the wrong thing" plus
/// a board is a puzzle; "it did X, I expected Y" plus a board is a test.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Told {
    /// What happened, in the player's own words.
    pub happened: String,
    /// What they expected instead.
    pub expected: String,
}

/// Which build this was, and what it was running on.
///
/// Every field here is about the program, not about the person. A GPU
/// adapter string and a window size are what separate "the client is wrong"
/// from "this machine draws it differently", and neither says who is at the
/// keyboard.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Build {
    /// The crate version.
    pub version: String,
    /// The commit, when the build was told one.
    pub commit: Option<String>,
    /// `target_os`/`target_arch`, or a browser's user agent on wasm.
    pub platform: String,
    /// What the renderer picked, verbatim.
    pub adapter: Option<String>,
    /// Logical window size.
    pub window: (u32, u32),
    /// Device pixels per logical pixel.
    pub scale: f32,
    /// The language the interface was speaking.
    pub lang: String,
}

/// Where in a game the report was written.
///
/// `seat` is a **number**, which is the whole point of this struct existing
/// rather than the fields being loose: the thing next to a seat number in
/// every other part of the client is the seat *token*, and the two must
/// never be typed into the same place by mistake. There is no field for it
/// here and there is no reason for one — the operator resolves a game and a
/// seat number against their own records.
#[derive(Clone, Debug, Serialize)]
pub struct Table {
    /// The gateway's id for the game, when this was not an offline duel.
    pub game: Option<String>,
    /// Which seat the reporter was sitting in.
    pub seat: u8,
    /// The view the report was written against, so the operator can line it
    /// up with the engine's own record of the same moment.
    pub seq: u64,
    /// Turn number, phase and step, in the interface's own spelling.
    pub when: String,
}

/// What the client was in the middle of.
///
/// The three states `dev-control` reports for the same reason a player
/// cannot see them: an action built and never sent, a run of lands being
/// tapped, an ability chooser holding the keyboard. All three look exactly
/// like "the key did nothing", which is how most of these reports start.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Holding {
    /// Objects the player had selected towards the current answer.
    pub selected: usize,
    /// The armed deed, rendered — a `Play`, an `Ability` or a mana `Run`.
    pub armed: Option<String>,
    /// Whether the client was tapping lands on the player's behalf.
    pub mana_run: bool,
    /// Actions built and not yet sent.
    pub outbox: usize,
    /// The engine's refusal of the last action, verbatim.
    pub last_error: Option<String>,
}

/// One player-visible note the client made, with the frame it made it on.
#[derive(Clone, Debug, Serialize)]
pub struct Note {
    /// The frame it was written on.
    pub frame: u64,
    /// The note.
    pub text: String,
}

/// Everything a report carries.
///
/// Assembled field by field on purpose — see the module header. A field
/// added here is a decision about what leaves a player's machine, which is
/// why there is no catch-all and why [`seal`] exists.
#[derive(Clone, Debug, Serialize)]
pub struct BugReport {
    /// What the player said.
    pub told: Told,
    /// Which build, and on what.
    pub build: Build,
    /// Where in a game, when there was one.
    pub table: Option<Table>,
    /// The whole board as this seat was shown it.
    ///
    /// Attached whole, and that is the point: what the reporter can see is
    /// exactly what this carries, so there is nothing here to redact.
    pub view: Option<PlayerView>,
    /// The question the engine was asking.
    pub pending: Option<Pending>,
    /// What the client was in the middle of.
    pub holding: Holding,
    /// The tail of the client's own notes.
    pub notes: Vec<Note>,
}

impl BugReport {
    /// An empty report carrying only what the player typed and which build
    /// they typed it in.
    #[must_use]
    pub fn new(told: Told, build: Build) -> Self {
        Self {
            told,
            build,
            table: None,
            view: None,
            pending: None,
            holding: Holding::default(),
            notes: Vec::new(),
        }
    }

    /// Attaches the game this was written during.
    #[must_use]
    pub fn at(mut self, table: Table, view: PlayerView, pending: Option<Pending>) -> Self {
        self.table = Some(table);
        self.view = Some(view);
        self.pending = pending;
        self
    }

    /// Attaches what the client was in the middle of.
    #[must_use]
    pub fn holding(mut self, holding: Holding) -> Self {
        self.holding = holding;
        self
    }

    /// Attaches the tail of the client's notes.
    #[must_use]
    pub fn with_notes(mut self, notes: Vec<Note>) -> Self {
        self.notes = notes;
        self
    }
}

/// The report as bytes, or a refusal because one of `secrets` is in it.
///
/// This is the only way to get a report out of this module, and the only
/// reason it is: "the struct has no field for a token" is a claim about
/// today's struct, and a report is assembled from half a dozen places that
/// each grow. Serialising first and searching the bytes afterwards is the
/// one check that stays true when someone adds a field — it does not care
/// *where* the secret got in, only that it is not going out.
///
/// A secret shorter than [`SHORTEST_SECRET`] is not searched for: see that
/// constant. An empty one is skipped for the same reason.
///
/// # Errors
///
/// [`Leaked`] when one of `secrets` appears in the serialised report, naming
/// the secret's label and never its value.
///
/// # Panics
///
/// Never: every field of a [`BugReport`] serialises, and the view and the
/// pending question are the wire types the client already receives as JSON.
pub fn seal(report: &BugReport, secrets: &[Secret<'_>]) -> Result<String, Leaked> {
    let json = serde_json::to_string(report).expect("a bug report serialises");
    for secret in secrets {
        if secret.value.len() < SHORTEST_SECRET {
            continue;
        }
        if json.contains(secret.value) {
            return Err(Leaked {
                label: secret.label.to_string(),
            });
        }
    }
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build() -> Build {
        Build {
            version: "0.1.0".into(),
            commit: Some("deadbeef".into()),
            platform: "macos/aarch64".into(),
            adapter: Some("Apple M1 Max".into()),
            window: (1728, 1052),
            scale: 2.0,
            lang: "de".into(),
        }
    }

    fn told() -> Told {
        Told {
            happened: "Der Command Tower wurde nicht getappt".into(),
            expected: "Harabaz Druid haette gecastet werden koennen".into(),
        }
    }

    /// The ordinary case: a report with no secret in it comes back as bytes.
    #[test]
    fn a_clean_report_is_handed_over() {
        let report = BugReport::new(told(), build());
        let json = seal(
            &report,
            &[Secret {
                label: "seat token",
                value: "0123456789abcdef0123456789abcdef",
            }],
        )
        .expect("nothing to find");
        assert!(json.contains("Command Tower"), "the player's words survive");
    }

    /// The whole point of the function. A token that reached the report by
    /// *any* route — here the free text, which is the one field no schema
    /// can constrain, because a player may paste anything into it — refuses
    /// the report instead of sending it.
    #[test]
    fn a_secret_anywhere_in_it_refuses_the_report() {
        let token = "0123456789abcdef0123456789abcdef";
        let mut report = BugReport::new(told(), build());
        report.told.happened = format!("ich war auf dem Tisch mit token={token}");

        let refused = seal(
            &report,
            &[Secret {
                label: "seat token",
                value: token,
            }],
        )
        .expect_err("the token is in there");
        assert_eq!(refused.label, "seat token");
        assert!(
            !refused.to_string().contains(token),
            "a refusal that quoted the secret would be a way to read it"
        );
    }

    /// And it searches every field, not the text ones: a secret that arrived
    /// through the build metadata — a gateway URL with a token in its query,
    /// which is exactly how one would get there — is caught the same way.
    #[test]
    fn a_secret_in_a_structured_field_is_found_too() {
        let token = "tok_aaaabbbbccccdddd";
        let mut report = BugReport::new(told(), build());
        report.build.adapter = Some(format!("http://gateway/?token={token}"));
        assert_eq!(
            seal(
                &report,
                &[Secret {
                    label: "session token",
                    value: token
                }]
            ),
            Err(Leaked {
                label: "session token".into()
            })
        );
    }

    /// A short "secret" is not one, and searching for it would refuse every
    /// report a player ever wrote: `"de"` is in the language field of all of
    /// them.
    #[test]
    fn something_too_short_to_be_a_secret_is_not_searched_for() {
        let report = BugReport::new(told(), build());
        seal(
            &report,
            &[
                Secret {
                    label: "language",
                    value: "de",
                },
                Secret {
                    label: "nothing at all",
                    value: "",
                },
            ],
        )
        .expect("neither is a secret");
    }
}
