//! The seat agent's link to the gateway (`GET /seathost/ws`,
//! `docs/protocol.md` §"Hosted language-model seats").
//!
//! JSON text frames, one [`Frame`] each, tagged by `"type"`. Not `Envelope`
//! entries: this is a control plane between two builds of one workspace
//! that the gateway forwards to nobody, so it touches neither the player
//! protocol nor [`crate::PROTOCOL_VERSION`] (which the hello still states,
//! and the gateway still checks).
//!
//! A profile's settings ([`Definition::profile`]) ride as JSON the seat
//! agent reads into the settings file's own type: this crate does not know
//! the settings file, and the gateway never reads them.

use serde::{Deserialize, Serialize};

/// The route a seat agent dials.
pub const PATH: &str = "/seathost/ws";

/// Whether `name` may name a seat agent or a profile: 1–64 of
/// `A-Za-z0-9._-`, not dots alone, so it goes into a path, a log field and
/// a file name as it is: `.` and `..` would be a path segment that walks
/// (a URL normalised to the parent route, a file name to the parent
/// directory). The feedback service, which links no protocol crate, keeps
/// the same rule in `console::llm_name`.
#[must_use]
pub fn valid_name(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
        && !name.bytes().all(|b| b == b'.')
}

/// Whether `text` may be a profile's label or vendor: 1–40 characters,
/// none a control character, not blank.
#[must_use]
pub fn valid_label(text: &str) -> bool {
    let count = text.chars().count();
    (1..=40).contains(&count) && !text.trim().is_empty() && !text.chars().any(char::is_control)
}

/// The name a hosted chair sits under after `LLM-`: the label's letters,
/// digits, `-` and `_` (a space as `-`), at most 12, a letter or digit at
/// each end, so `LLM-…` is a display name the gateway takes; else the
/// profile id's, else `model`.
#[must_use]
pub fn chair_name(label: &str, id: &str) -> String {
    let fit = |text: &str| {
        let kept: String = text
            .chars()
            .map(|c| if c == ' ' { '-' } else { c })
            .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
            .take(12)
            .collect();
        let kept = kept.trim_matches(|c: char| !c.is_ascii_alphanumeric());
        (!kept.is_empty()).then(|| kept.to_string())
    };
    fit(label)
        .or_else(|| fit(id))
        .unwrap_or_else(|| "model".into())
}

/// One frame on the link.
///
/// @description One frame on the seat agents' socket, one JSON object per
/// text frame, tagged by `type`.
/// @openapi
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Frame {
    /// @description Seat agent to gateway, first: who it is.
    /// @internal
    /// Agent → gateway, first: who it is.
    Hello {
        /// @description The gateway's `BAYLEE_SEATHOST_TOKEN`.
        /// @internal
        /// `BAYLEE_SEATHOST_TOKEN`.
        token: String,
        /// @description Its stable name: 1 to 64 of `A-Z a-z 0-9 . - _`,
        /// not dots alone.
        /// @internal
        /// Its stable name ([`valid_name`]).
        name: String,
        /// @description The protocol it was built with; another is refused.
        /// @internal
        /// The protocol it was built with.
        protocol_version: u32,
        /// @description How many bridges it runs at once; 0 means no bound.
        /// @internal
        /// Live bridges at once; 0 = no bound.
        capacity: u32,
    },
    /// @description Gateway to seat agent: accepted.
    /// @internal
    /// Gateway → agent: accepted.
    Welcome {
        /// @description How often, in seconds, to say it is there
        /// (`heartbeat`); two missed and it is dropped.
        /// @internal
        /// How often to say it is there.
        heartbeat_secs: u32,
    },
    /// @description Gateway to seat agent: refused, and why; the socket
    /// closes.
    /// @internal
    /// Gateway → agent: refused, and why; the socket closes.
    Refused {
        /// @description One sentence.
        message: String,
    },
    /// @description Seat agent to gateway: every profile it holds, whole,
    /// on connect and after every change.
    /// @internal
    /// Agent → gateway: every profile it holds, whole.
    Profiles {
        /// @description The profiles.
        profiles: Vec<Reported>,
    },
    /// @description Gateway to seat agent: start a bridge for one chair.
    /// @internal
    /// Gateway → agent: start a bridge for one chair.
    StartSeat {
        /// @description The gateway's id for this order.
        order: String,
        /// @description The room.
        game_id: String,
        /// @description The chair.
        seat: u32,
        /// @description The profile id.
        profile: String,
        /// @description The chair ticket the bridge redeems; handed to it
        /// as its first line of standard input, never logged.
        /// @internal
        /// The chair ticket, for the bridge's stdin; never logged.
        chair_ticket: String,
        /// @description Where the bridge dials.
        gateway_url: String,
        /// @description A deck in any format the bridge reads; absent for
        /// the bridge's own.
        /// @internal
        /// A deck in any format the bridge reads, or none for its own.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        deck_text: Option<String>,
    },
    /// @description Gateway to seat agent: stop an order's bridge.
    /// @internal
    /// Gateway → agent: stop an order's bridge.
    StopSeat {
        /// @description The order.
        order: String,
    },
    /// @description Seat agent to gateway: what became of an order.
    /// @internal
    /// Agent → gateway: what became of an order.
    SeatStatus {
        /// @description The order.
        order: String,
        /// @description What became of it.
        /// @internal
        /// What.
        kind: SeatStatusKind,
        /// @description One sentence, never a key; empty when absent.
        /// @internal
        /// One sentence, never a key.
        #[serde(default)]
        detail: String,
    },
    /// @description Gateway to seat agent: write a profile's definition;
    /// answered by `answer` with the same `request`.
    /// @internal
    /// Gateway → agent: write a profile's definition.
    ProfileWrite {
        /// @description The request's id, which the `answer` names.
        /// @internal
        /// Answered by [`Frame::Answer`] with the same id.
        request: String,
        /// @description The profile id.
        id: String,
        /// @description What the profile is.
        /// @internal
        /// What it is.
        definition: Definition,
        /// @description The admin who asked.
        admin: String,
    },
    /// @description Gateway to seat agent: forget a profile; answered by
    /// `answer`.
    /// @internal
    /// Gateway → agent: forget a profile.
    ProfileDelete {
        /// @description The request's id, which the `answer` names.
        /// @internal
        /// Answered by [`Frame::Answer`].
        request: String,
        /// @description The profile id.
        id: String,
        /// @description The admin who asked.
        admin: String,
    },
    /// @description Gateway to seat agent: act on a profile; answered by
    /// `answer`.
    /// @internal
    /// Gateway → agent: act on a profile.
    Control {
        /// @description The request's id, which the `answer` names.
        /// @internal
        /// Answered by [`Frame::Answer`].
        request: String,
        /// @description The profile id.
        id: String,
        /// @description What to do.
        /// @internal
        /// What.
        action: ControlAction,
        /// @description The admin who asked.
        admin: String,
    },
    /// @description Gateway to seat agent: keep (or forget) a profile's
    /// key; answered by `answer`. Write-only: nothing ever answers a key.
    /// Only a seat agent on the gateway's unix socket is sent one.
    /// @internal
    /// Gateway → agent: keep (or forget) a profile's key. Write-only:
    /// nothing ever answers a key.
    KeyWrite {
        /// @description The request's id, which the `answer` names.
        /// @internal
        /// Answered by [`Frame::Answer`].
        request: String,
        /// @description The profile id.
        id: String,
        /// @description The key; `null` or absent forgets it.
        /// @internal
        /// The key; `None` forgets it.
        #[serde(default)]
        key: Option<String>,
        /// @description The admin who asked.
        admin: String,
    },
    /// @description Seat agent to gateway: how a request went.
    /// @internal
    /// Agent → gateway: how a request went.
    Answer {
        /// @description The request's id.
        request: String,
        /// @description Whether it was done.
        ok: bool,
        /// @description Why not, one sentence; absent when done.
        /// @internal
        /// Why not, one sentence.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    /// @description Either way: still here.
    /// @internal
    /// Either way: still here.
    Heartbeat,
}

impl std::fmt::Debug for Frame {
    /// Names the frame and never a secret: a token, a ticket or a key is
    /// left out, so a frame in a log line or a panic says nothing it holds.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hello { name, .. } => write!(f, "Hello({name})"),
            Self::StartSeat {
                order,
                game_id,
                seat,
                profile,
                ..
            } => write!(f, "StartSeat({order}, {game_id}#{seat}, {profile})"),
            Self::KeyWrite { request, id, .. } => write!(f, "KeyWrite({request}, {id})"),
            Self::Welcome { .. } => f.write_str("Welcome"),
            Self::Refused { .. } => f.write_str("Refused"),
            Self::Profiles { profiles } => write!(f, "Profiles({})", profiles.len()),
            Self::StopSeat { order } => write!(f, "StopSeat({order})"),
            Self::SeatStatus { order, kind, .. } => write!(f, "SeatStatus({order}, {kind:?})"),
            Self::ProfileWrite { request, id, .. } => write!(f, "ProfileWrite({request}, {id})"),
            Self::ProfileDelete { request, id, .. } => {
                write!(f, "ProfileDelete({request}, {id})")
            }
            Self::Control {
                request,
                id,
                action,
                ..
            } => write!(f, "Control({request}, {id}, {action:?})"),
            Self::Answer { request, ok, .. } => write!(f, "Answer({request}, {ok})"),
            Self::Heartbeat => f.write_str("Heartbeat"),
        }
    }
}

impl Frame {
    /// The frame as a text frame's payload.
    #[must_use]
    pub fn to_text(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{\"type\":\"heartbeat\"}".into())
    }

    /// A text frame's payload read.
    ///
    /// # Errors
    /// Not a frame of this protocol.
    pub fn parse(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| format!("not a seat agent frame: {e}"))
    }
}

/// What became of an order.
///
/// @description What became of an order: `started` (the bridge is
/// running), `failed` (it never got going, or the profile could not take
/// it) or `exited` (it ended).
/// @openapi
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeatStatusKind {
    /// The bridge is running.
    Started,
    /// It never got going, or the profile could not take it.
    Failed,
    /// It ended.
    Exited,
}

/// An action on a profile.
///
/// @description An action on a profile: `probe` (run its check now),
/// `enable` or `disable`.
/// @openapi
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlAction {
    /// Run its check now.
    Probe,
    /// Switch it on.
    Enable,
    /// Switch it off.
    Disable,
}

/// What a profile is, as an admin writes it: nothing secret.
///
/// @description A hosted profile's definition, as an admin writes it:
/// nothing secret (a key is written apart). The seat agent checks it and
/// answers its refusal in a sentence.
/// @openapi
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    /// @description What players see it called.
    /// @internal
    /// What players see ([`valid_label`]).
    pub label: String,
    /// @description Who the game data goes to.
    /// @internal
    /// Who the game data goes to ([`valid_label`]).
    pub vendor: String,
    /// @description Whether it is offered at all; `true` when absent.
    /// @internal
    /// Offered at all.
    #[serde(default = "yes")]
    pub enabled: bool,
    /// @description How many games it plays at once; no bound when absent.
    /// @internal
    /// Games at once; `None` = no bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_games: Option<u32>,
    /// @description Its own spend caps, in the seat settings file's
    /// format (`docs/llm-seat.md`); none when absent.
    /// @internal
    /// The profile's own spend caps; `None` = none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caps: Option<serde_json::Value>,
    /// @description Whether each probe also makes a paid canary call.
    /// @internal
    /// A paid canary call with each probe.
    #[serde(default)]
    pub canary: bool,
    /// @description Other profiles of the same seat agent and vendor that
    /// take a game over, one after the other, when the model playing is
    /// rate limited, out of its quota or failing, and that new games start
    /// on while this one is not available: one profile's id, or a list of
    /// them in the order they take over; none when absent.
    /// @internal
    /// Other profiles of the same seat agent that take a game over in turn
    /// when the model playing is rate limited or fails, and the first
    /// available of which a new game starts on while this one is not
    /// available (at `max_games`, limited, failing, …): each a model behind
    /// an API, this one behind an API or an agent CLI. `None` = none. A
    /// fallback's own fallbacks are not followed (`docs/protocol.md` §"A
    /// fallback profile").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<Fallbacks>,
    /// @description One profile of the seat settings file, in its own
    /// format (`docs/llm-seat.md`).
    /// @internal
    /// One profile of the settings file, in its own format.
    pub profile: serde_json::Value,
}

impl Definition {
    /// The fallbacks it names, in the order they take over; none where it
    /// names none.
    #[must_use]
    pub fn fallbacks(&self) -> &[String] {
        self.fallback.as_ref().map_or(&[], Fallbacks::ids)
    }
}

/// The fallbacks a definition names: one profile's id, as a definition with
/// one fallback has always said it, or several in the order they take
/// over.
///
/// @description One profile's id, or a list of them in the order they take
/// over.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Fallbacks {
    /// @description One profile's id.
    /// @internal
    /// One profile, written as its id alone.
    One(String),
    /// @description Profiles' ids, in the order they take over.
    /// @internal
    /// Several, in the order they take over.
    Chain(Vec<String>),
}

impl Fallbacks {
    /// The ids, in the order they take over.
    #[must_use]
    pub fn ids(&self) -> &[String] {
        match self {
            Self::One(id) => std::slice::from_ref(id),
            Self::Chain(ids) => ids,
        }
    }
}

/// One id as [`Fallbacks::One`], so that one fallback is written as it
/// always was; any other number as a list.
impl From<Vec<String>> for Fallbacks {
    fn from(mut ids: Vec<String>) -> Self {
        if ids.len() == 1 {
            Self::One(ids.remove(0))
        } else {
            Self::Chain(ids)
        }
    }
}

impl From<&str> for Fallbacks {
    fn from(id: &str) -> Self {
        Self::One(id.to_owned())
    }
}

const fn yes() -> bool {
    true
}

/// A profile's state (`docs/protocol.md` §"Hosted language-model seats").
///
/// @description A profile's state: `available`, `busy` (at its games'
/// bound, or its seat agent at capacity), `probing` (its check is running),
/// `exhausted` (a spend cap is reached or the provider said it is limited),
/// `needs_login` (a CLI's login check says signed out), `failing` (its
/// check failed) or `disabled` (switched off).
/// @openapi
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// Offered.
    Available,
    /// At its games' bound, or its seat agent at capacity.
    Busy,
    /// Its check is running.
    Probing,
    /// A cap reached or the provider limited.
    Exhausted,
    /// A CLI signed out.
    NeedsLogin,
    /// Its check failed.
    Failing,
    /// Switched off.
    Disabled,
}

impl State {
    /// The word on the wire.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Busy => "busy",
            Self::Probing => "probing",
            Self::Exhausted => "exhausted",
            Self::NeedsLogin => "needs_login",
            Self::Failing => "failing",
            Self::Disabled => "disabled",
        }
    }
}

/// Whether a key is kept for a profile; never the key.
///
/// @description Whether a key is kept for a profile, never the key:
/// `kept`, `absent`, `none_needed` (a CLI, or an endpoint that takes none)
/// or `unavailable` (the seat agent has no key store).
/// @openapi
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyKept {
    /// One is kept.
    Kept,
    /// None is.
    Absent,
    /// The profile takes none.
    NoneNeeded,
    /// The seat agent has no store.
    Unavailable,
}

/// What a profile spent in its own book.
///
/// @description What a profile spent in its own spend book.
/// @openapi
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Spent {
    /// @description US dollars today (UTC).
    /// @internal
    /// Dollars today (UTC).
    pub day_usd: f64,
    /// @description US dollars this month (UTC).
    /// @internal
    /// Dollars this month.
    pub month_usd: f64,
    /// @description Tokens today (UTC).
    /// @internal
    /// Tokens today.
    pub day_tokens: u64,
    /// @description Tokens this month (UTC).
    /// @internal
    /// Tokens this month.
    pub month_tokens: u64,
}

/// A profile as a seat agent reports it.
///
/// @description A profile as its seat agent reports it. The gateway
/// derives none of it.
/// @openapi
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reported {
    /// @description Its id.
    pub id: String,
    /// @description What players see it called.
    /// @internal
    /// What players see.
    pub label: String,
    /// @description Who the game data goes to.
    pub vendor: String,
    /// @description `api` (a model behind an HTTP API and a key) or `cli`
    /// (an agent CLI signed in on the seat agent's machine).
    /// @openapi enum: [api, cli]
    /// @internal
    /// `api` or `cli`.
    pub kind: String,
    /// @description The model, as the profile names it.
    pub model: String,
    /// @description Its state.
    pub state: State,
    /// @description When it is expected back, where that is known, in Unix
    /// seconds.
    /// @internal
    /// When it is expected back, where known (Unix seconds).
    #[serde(default)]
    pub until_unix: Option<i64>,
    /// @description How many of its bridges are live.
    /// @internal
    /// Live bridges of it.
    pub games: u32,
    /// @description How many games it plays at once; no bound when `null`.
    /// @internal
    /// Its bound.
    #[serde(default)]
    pub max_games: Option<u32>,
    /// @description Whether it is switched on.
    /// @internal
    /// Switched on.
    pub enabled: bool,
    /// @description Whether each probe also makes a paid canary call.
    /// @internal
    /// Probes pay a canary call.
    #[serde(default)]
    pub canary: bool,
    /// @description Its spend caps, as its definition says; none when
    /// `null`.
    /// @internal
    /// Its caps, as the definition says.
    #[serde(default)]
    pub caps: Option<serde_json::Value>,
    /// @description What it spent today and this month.
    /// @internal
    /// What it spent.
    #[serde(default)]
    pub spent: Spent,
    /// @description Whether a key is kept.
    pub key: KeyKept,
    /// @description The last failure, one sentence; `null` when none.
    /// @internal
    /// The last failure, one sentence.
    #[serde(default)]
    pub last_error: Option<String>,
    /// @description The last success, in Unix seconds.
    /// @internal
    /// The last success (Unix seconds).
    #[serde(default)]
    pub last_ok_unix: Option<i64>,
    /// @description The fallback profile a new game of this one starts on
    /// now, because this one is not available and that fallback is (the
    /// first of its fallbacks that is); the profile is then reported
    /// `available`. Absent otherwise.
    /// @internal
    /// The fallback a new game of this profile starts on now, because the
    /// profile is not available (at `max_games`, limited, failing, signed
    /// out, being checked, switched off) and that fallback, the first
    /// available of its chain, is:
    /// the profile is then reported `available`, with its own games and
    /// bound (`docs/protocol.md` §"A fallback profile"). `None` = a new
    /// game plays the profile itself, or not at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diverts: Option<String>,
    /// @description What an admin wrote.
    pub definition: Definition,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_reads_back_and_its_debug_holds_no_secret() {
        let frame = Frame::KeyWrite {
            request: "r1".into(),
            id: "sonnet".into(),
            key: Some("sk-ant-secret-value".into()),
            admin: "ada".into(),
        };
        let text = frame.to_text();
        assert!(text.contains("\"type\":\"key_write\""), "{text}");
        assert_eq!(Frame::parse(&text).unwrap(), frame);
        assert!(!format!("{frame:?}").contains("secret"));
        let start = Frame::StartSeat {
            order: "o".into(),
            game_id: "g".into(),
            seat: 1,
            profile: "p".into(),
            chair_ticket: "the-ticket".into(),
            gateway_url: "http://x".into(),
            deck_text: None,
        };
        assert!(!format!("{start:?}").contains("the-ticket"));
        assert_eq!(
            Frame::parse(r#"{"type":"heartbeat"}"#).unwrap(),
            Frame::Heartbeat
        );
    }

    #[test]
    fn names_and_labels() {
        assert!(valid_name("sonnet-5.5_x"));
        assert!(!valid_name(""));
        assert!(!valid_name("a/b"));
        assert!(!valid_name(&"a".repeat(65)));
        // Dots alone walk a path; a dot inside a name does not.
        assert!(!valid_name("."));
        assert!(!valid_name(".."));
        assert!(!valid_name("..."));
        assert!(valid_name(".hidden"));
        assert!(valid_name("a..b"));
        assert!(valid_label("Claude Sonnet"));
        assert!(!valid_label("  "));
        assert!(!valid_label("a\nb"));
        assert_eq!(chair_name("Claude Sonnet 5.5", "x"), "Claude-Sonne");
        assert_eq!(chair_name("—", "opus_1"), "opus_1");
        assert_eq!(chair_name("…", "."), "model");
    }

    /// A definition's `fallback` is one id, as it always was, or a list of
    /// them in order; one is written back as one, and nothing else is read.
    #[test]
    fn a_definition_names_one_fallback_or_a_chain() {
        let read = |fallback: &str| {
            serde_json::from_str::<Definition>(&format!(
                r#"{{"label": "L", "vendor": "V", "profile": {{}}, "fallback": {fallback}}}"#
            ))
        };
        let one = read(r#""backup""#).unwrap();
        assert_eq!(one.fallback, Some(Fallbacks::One("backup".into())));
        assert_eq!(one.fallbacks(), ["backup"]);
        assert_eq!(serde_json::to_value(&one).unwrap()["fallback"], "backup");
        let chain = read(r#"["a", "b"]"#).unwrap();
        assert_eq!(chain.fallbacks(), ["a", "b"]);
        assert_eq!(
            serde_json::to_value(&chain).unwrap()["fallback"],
            serde_json::json!(["a", "b"])
        );
        assert!(read("3").is_err() && read(r#"{"id": "a"}"#).is_err());
        let none = Definition {
            fallback: None,
            ..one.clone()
        };
        assert!(none.fallbacks().is_empty());
        assert!(!serde_json::to_string(&none).unwrap().contains("fallback"));
        assert_eq!(
            Fallbacks::from(vec!["only".to_owned()]),
            Fallbacks::One("only".into())
        );
        assert_eq!(
            Fallbacks::from(vec!["a".to_owned(), "b".to_owned()]).ids(),
            ["a", "b"]
        );
    }

    #[test]
    fn states_order_best_first() {
        assert!(State::Available < State::Busy);
        assert!(State::Exhausted < State::Disabled);
        assert_eq!(State::NeedsLogin.word(), "needs_login");
    }
}
