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
/// `A-Za-z0-9._-`, so it goes into a path, a log field and a file name
/// as it is.
#[must_use]
pub fn valid_name(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

/// Whether `text` may be a profile's label or vendor: 1–40 characters,
/// none a control character, not blank.
#[must_use]
pub fn valid_label(text: &str) -> bool {
    let count = text.chars().count();
    (1..=40).contains(&count) && !text.trim().is_empty() && !text.chars().any(char::is_control)
}

/// One frame on the link.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Frame {
    /// Agent → gateway, first: who it is.
    Hello {
        /// `BAYLEE_SEATHOST_TOKEN`.
        token: String,
        /// Its stable name ([`valid_name`]).
        name: String,
        /// The protocol it was built with.
        protocol_version: u32,
        /// Live bridges at once; 0 = no bound.
        capacity: u32,
    },
    /// Gateway → agent: accepted.
    Welcome {
        /// How often to say it is there.
        heartbeat_secs: u32,
    },
    /// Gateway → agent: refused, and why; the socket closes.
    Refused {
        /// One sentence.
        message: String,
    },
    /// Agent → gateway: every profile it holds, whole.
    Profiles {
        /// The profiles.
        profiles: Vec<Reported>,
    },
    /// Gateway → agent: start a bridge for one chair.
    StartSeat {
        /// The gateway's id for this order.
        order: String,
        /// The room.
        game_id: String,
        /// The chair.
        seat: u32,
        /// The profile id.
        profile: String,
        /// The chair ticket, for the bridge's stdin; never logged.
        chair_ticket: String,
        /// Where the bridge dials.
        gateway_url: String,
        /// A deck in any format the bridge reads, or none for its own.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        deck_text: Option<String>,
    },
    /// Gateway → agent: stop an order's bridge.
    StopSeat {
        /// The order.
        order: String,
    },
    /// Agent → gateway: what became of an order.
    SeatStatus {
        /// The order.
        order: String,
        /// What.
        kind: SeatStatusKind,
        /// One sentence, never a key.
        #[serde(default)]
        detail: String,
    },
    /// Gateway → agent: write a profile's definition.
    ProfileWrite {
        /// Answered by [`Frame::Answer`] with the same id.
        request: String,
        /// The profile id.
        id: String,
        /// What it is.
        definition: Definition,
        /// The admin who asked.
        admin: String,
    },
    /// Gateway → agent: forget a profile.
    ProfileDelete {
        /// Answered by [`Frame::Answer`].
        request: String,
        /// The profile id.
        id: String,
        /// The admin who asked.
        admin: String,
    },
    /// Gateway → agent: act on a profile.
    Control {
        /// Answered by [`Frame::Answer`].
        request: String,
        /// The profile id.
        id: String,
        /// What.
        action: ControlAction,
        /// The admin who asked.
        admin: String,
    },
    /// Gateway → agent: keep (or forget) a profile's key. Write-only:
    /// nothing ever answers a key.
    KeyWrite {
        /// Answered by [`Frame::Answer`].
        request: String,
        /// The profile id.
        id: String,
        /// The key; `None` forgets it.
        #[serde(default)]
        key: Option<String>,
        /// The admin who asked.
        admin: String,
    },
    /// Agent → gateway: how a request went.
    Answer {
        /// The request's id.
        request: String,
        /// Whether it was done.
        ok: bool,
        /// Why not, one sentence.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    /// What players see ([`valid_label`]).
    pub label: String,
    /// Who the game data goes to ([`valid_label`]).
    pub vendor: String,
    /// Offered at all.
    #[serde(default = "yes")]
    pub enabled: bool,
    /// Games at once; `None` = no bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_games: Option<u32>,
    /// The profile's own spend caps; `None` = none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caps: Option<serde_json::Value>,
    /// A paid canary call with each probe.
    #[serde(default)]
    pub canary: bool,
    /// One profile of the settings file, in its own format.
    pub profile: serde_json::Value,
}

const fn yes() -> bool {
    true
}

/// A profile's state (`docs/protocol.md` §"Hosted language-model seats").
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Spent {
    /// Dollars today (UTC).
    pub day_usd: f64,
    /// Dollars this month.
    pub month_usd: f64,
    /// Tokens today.
    pub day_tokens: u64,
    /// Tokens this month.
    pub month_tokens: u64,
}

/// A profile as a seat agent reports it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reported {
    /// Its id.
    pub id: String,
    /// What players see.
    pub label: String,
    /// Who the game data goes to.
    pub vendor: String,
    /// `api` or `cli`.
    pub kind: String,
    /// The model, as the profile names it.
    pub model: String,
    /// Its state.
    pub state: State,
    /// When it is expected back, where known (Unix seconds).
    #[serde(default)]
    pub until_unix: Option<i64>,
    /// Live bridges of it.
    pub games: u32,
    /// Its bound.
    #[serde(default)]
    pub max_games: Option<u32>,
    /// Switched on.
    pub enabled: bool,
    /// Probes pay a canary call.
    #[serde(default)]
    pub canary: bool,
    /// Its caps, as the definition says.
    #[serde(default)]
    pub caps: Option<serde_json::Value>,
    /// What it spent.
    #[serde(default)]
    pub spent: Spent,
    /// Whether a key is kept.
    pub key: KeyKept,
    /// The last failure, one sentence.
    #[serde(default)]
    pub last_error: Option<String>,
    /// The last success (Unix seconds).
    #[serde(default)]
    pub last_ok_unix: Option<i64>,
    /// What an admin wrote.
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
        assert!(valid_label("Claude Sonnet"));
        assert!(!valid_label("  "));
        assert!(!valid_label("a\nb"));
    }

    #[test]
    fn states_order_best_first() {
        assert!(State::Available < State::Busy);
        assert!(State::Exhausted < State::Disabled);
        assert_eq!(State::NeedsLogin.word(), "needs_login");
    }
}
