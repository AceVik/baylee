//! Friends, presence and end-to-end encrypted chat (`GET /social/ws` and
//! `/social/…`, `docs/protocol.md` §"Friends and chat (beta.8)").
//!
//! JSON text frames, one [`Up`] or [`Down`] each, tagged by `"t"`. Not
//! `Envelope` entries: nothing here reaches the engine, so the player
//! protocol and [`crate::PROTOCOL_VERSION`] do not move. The socket still
//! states the protocol (and the gateway still refuses a mismatch with
//! [`crate::version_refusal`]), and it states its own [`SOCIAL_VERSION`] too.
//!
//! Every message body is sealed on the sender's device for the recipients'
//! devices (`baylee-seal`); the gateway carries [`Sealed`] and [`Delivered`]
//! as opaque base64url strings and never holds a key that opens them. Binary
//! values are base64url without padding, field names `snake_case`.
//!
//! The public identifier of an account is its **tag** (`af03`, lowercase hex,
//! at least four digits): unique, never changing, already public in every
//! handle. Account ids never leave the gateway.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// The social protocol this build speaks. A socket that states another is
/// refused with one [`Down::Refused`] (`why: protocol`) and closed.
pub const SOCIAL_VERSION: u32 = 1;

/// The route the social socket is opened on.
pub const PATH: &str = "/social/ws";

/// The `socket` a client names in `POST /ws-ticket` for the social socket:
/// `{"socket":"social"}`.
pub const TICKET_SOCKET: &str = "social";

/// The path and query the social socket is opened on, relative to the
/// gateway: `/social/ws?ticket=…&protocol=…&social=…`.
///
/// Every dialer builds it here, so none leaves out a version. It takes a
/// ticket from [`crate::WS_TICKET_PATH`], never the session token.
#[must_use]
pub fn social_socket_path(ticket: &str) -> String {
    format!(
        "{PATH}?ticket={ticket}&protocol={}&social={SOCIAL_VERSION}",
        crate::PROTOCOL_VERSION
    )
}

/// Why a social socket that states `theirs` is refused, or `None` when it
/// speaks this build's [`SOCIAL_VERSION`]. The same shape of sentence as
/// [`crate::version_refusal`].
#[must_use]
pub fn social_version_refusal(who: &str, theirs: u32) -> Option<String> {
    (theirs != SOCIAL_VERSION).then(|| {
        if theirs == 0 {
            format!(
                "{who} does not say which chat protocol it speaks; this gateway speaks {SOCIAL_VERSION}."
            )
        } else {
            format!("{who} speaks chat protocol {theirs}; this gateway speaks {SOCIAL_VERSION}.")
        }
    })
}

// ---------------------------------------------------------------------------
// Limits (§8.5): both ends check them.
// ---------------------------------------------------------------------------

/// The most characters a message's text has.
pub const MAX_TEXT_CHARS: usize = 1000;
/// The most bytes a message's plaintext body (JSON) has.
pub const MAX_BODY_BYTES: usize = 4096;
/// The most bytes a message's header (JSON) has.
pub const MAX_HEADER_BYTES: usize = 512;
/// The most bytes one client frame has; a larger one closes the socket with
/// 1008. Frames the gateway sends are not bounded by it.
pub const MAX_SOCIAL_FRAME: usize = 48 * 1024;
/// The most chat devices a registered account has.
pub const MAX_DEVICES: usize = 5;
/// The most chat devices a guest has; a new registration replaces the old.
pub const GUEST_DEVICES: usize = 1;
/// The most members a group has, pending invites included.
pub const MAX_GROUP_MEMBERS: usize = 50;
/// The most groups an account is in.
pub const MAX_GROUPS: usize = 20;
/// The most friends an account has.
pub const MAX_FRIENDS: usize = 300;
/// The most friend requests an account has outstanding.
pub const MAX_OUTGOING_REQUESTS: usize = 50;
/// The fewest characters a group's name has.
pub const GROUP_NAME_MIN: usize = 1;
/// The most characters a group's name has.
pub const GROUP_NAME_MAX: usize = 48;
/// How many days the gateway holds a message for a registered account's
/// device when the operator says nothing (`BAYLEE_SOCIAL_KEEP_DAYS`, 1..=30).
pub const KEEP_DAYS_DEFAULT: u32 = 14;
/// The lowest `BAYLEE_SOCIAL_KEEP_DAYS`.
pub const KEEP_DAYS_MIN: u32 = 1;
/// The highest `BAYLEE_SOCIAL_KEEP_DAYS`.
pub const KEEP_DAYS_MAX: u32 = 30;
/// How many days the gateway holds a message for a guest's device.
pub const GUEST_KEEP_DAYS: u32 = 1;
/// The most undelivered slots one device has; past it a send that would add
/// one is refused with [`Why::RecipientFull`].
pub const SLOTS_PER_DEVICE: usize = 5000;
/// How many pending deliveries the gateway sends after a snapshot before it
/// waits for acks (flow control).
pub const DELIVERY_WINDOW: usize = 256;
/// How far a header's `ts` may be from the gateway's clock, in milliseconds.
pub const MAX_CLOCK_SKEW_MS: u64 = 10 * 60 * 1000;
/// The most tags one `GET /social/keys` names.
pub const MAX_KEY_TAGS: usize = 60;
/// The most characters a device's label has (an OS family, `macOS`).
pub const MAX_DEVICE_LABEL: usize = 24;
/// The length of a device id: 16 lowercase hex characters.
pub const DEVICE_ID_LEN: usize = 16;
/// The length of a wrapped message key, in bytes.
pub const WRAP_BYTES: usize = 48;
/// How long a table invite stands, in seconds.
pub const TABLE_INVITE_SECS: u32 = 600;
/// The most characters a report's note has.
pub const MAX_REPORT_NOTE: usize = 2000;

// ---------------------------------------------------------------------------
// Tags, devices and conversation ids.
// ---------------------------------------------------------------------------

/// Whether `tag` is a tag as the gateway renders one: lowercase hex, four to
/// eight digits, no leading zero past the fourth digit.
#[must_use]
pub fn valid_tag(tag: &str) -> bool {
    (4..=8).contains(&tag.len())
        && tag.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        && (tag.len() == 4 || !tag.starts_with('0'))
}

/// Tags in the order a [`Conv::Dm`] names them: numeric, which for canonical
/// tags is by length, then by string.
#[must_use]
pub fn tag_order(a: &str, b: &str) -> std::cmp::Ordering {
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}

/// Whether `id` is a device id: 16 lowercase hex characters (§4.1).
#[must_use]
pub fn valid_device_id(id: &str) -> bool {
    id.len() == DEVICE_ID_LEN && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Whether `id` may name a game in a conversation id: 1–64 of
/// `A-Za-z0-9-_` (no `:`).
#[must_use]
pub fn valid_game_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

/// Whether `id` has a UUID's text shape, lowercase (`8-4-4-4-12` hex).
#[must_use]
pub fn valid_uuid(id: &str) -> bool {
    let bytes = id.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => *b == b'-',
            _ => matches!(b, b'0'..=b'9' | b'a'..=b'f'),
        })
}

/// A conversation, as its id names it (§5.1).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Conv {
    /// `dm:<tagA>:<tagB>`: a whisper between two accounts, tags in
    /// [`tag_order`]. Build one with [`Conv::dm`].
    Dm {
        /// The lower tag.
        a: String,
        /// The higher tag.
        b: String,
    },
    /// `grp:<uuid>`: a group.
    Group {
        /// The group's id.
        id: String,
    },
    /// `tbl:<game_id>`: the players with a chair at one table.
    Table {
        /// The lobby game's id.
        game: String,
    },
    /// `team:<game_id>:<n>`: the players of one team at one table.
    Team {
        /// The lobby game's id.
        game: String,
        /// The team's number.
        team: u8,
    },
    /// `global`: the public lobby chat for all online players.
    Global,
}

/// Why a string is not a [`Conv`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConvFault;

impl fmt::Display for ConvFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("not a conversation id")
    }
}

impl std::error::Error for ConvFault {}

impl Conv {
    /// The whisper between `x` and `y`, in either order; `None` when either
    /// is not a tag or both are one.
    #[must_use]
    pub fn dm(x: &str, y: &str) -> Option<Self> {
        if !valid_tag(x) || !valid_tag(y) || x == y {
            return None;
        }
        let (a, b) = if tag_order(x, y).is_lt() {
            (x, y)
        } else {
            (y, x)
        };
        Some(Self::Dm {
            a: a.to_string(),
            b: b.to_string(),
        })
    }

    /// For a whisper, the tag that is not `me`; `None` for any other
    /// conversation or one `me` is not in.
    #[must_use]
    pub fn partner(&self, me: &str) -> Option<&str> {
        match self {
            Self::Dm { a, b } if a == me => Some(b),
            Self::Dm { a, b } if b == me => Some(a),
            _ => None,
        }
    }

    /// The game a table or team conversation belongs to.
    #[must_use]
    pub fn game(&self) -> Option<&str> {
        match self {
            Self::Table { game } | Self::Team { game, .. } => Some(game),
            _ => None,
        }
    }
}

impl fmt::Display for Conv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dm { a, b } => write!(f, "dm:{a}:{b}"),
            Self::Group { id } => write!(f, "grp:{id}"),
            Self::Table { game } => write!(f, "tbl:{game}"),
            Self::Team { game, team } => write!(f, "team:{game}:{team}"),
            Self::Global => f.write_str("global"),
        }
    }
}

impl FromStr for Conv {
    type Err = ConvFault;

    /// Exactly the canonical text [`Display`](fmt::Display) writes: a
    /// whisper with its tags out of order, or a team number with a leading
    /// zero, is refused rather than read as another spelling of one id.
    fn from_str(text: &str) -> Result<Self, ConvFault> {
        if text == "global" {
            return Ok(Self::Global);
        }
        let (kind, rest) = text.split_once(':').ok_or(ConvFault)?;
        let conv = match kind {
            "dm" => {
                let (a, b) = rest.split_once(':').ok_or(ConvFault)?;
                let conv = Self::dm(a, b).ok_or(ConvFault)?;
                if conv.partner(a) != Some(b) || tag_order(a, b).is_gt() {
                    return Err(ConvFault);
                }
                conv
            }
            "grp" if valid_uuid(rest) => Self::Group { id: rest.into() },
            "tbl" if valid_game_id(rest) => Self::Table { game: rest.into() },
            "team" => {
                let (game, team) = rest.split_once(':').ok_or(ConvFault)?;
                let n: u8 = team.parse().map_err(|_| ConvFault)?;
                if !valid_game_id(game) || n.to_string() != team {
                    return Err(ConvFault);
                }
                Self::Team {
                    game: game.into(),
                    team: n,
                }
            }
            _ => return Err(ConvFault),
        };
        Ok(conv)
    }
}

impl Serialize for Conv {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Conv {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

// ---------------------------------------------------------------------------
// The message header (§4.2): the sender's JSON bytes, signed as bytes.
// ---------------------------------------------------------------------------

/// A message's header as the gateway and the recipient read it. It travels
/// as the sender's bytes (`Sealed::hdr`, base64url) and is signed as those
/// bytes, so there is no canonical form: parse it, never re-serialize it
/// to compare.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    /// The header's version: 1.
    pub v: u32,
    /// The message's id, a `UUIDv7` the sender makes.
    pub id: String,
    /// The conversation, as [`Conv`]'s text.
    pub conv: String,
    /// The sender's tag.
    pub from: String,
    /// The sending device's id.
    pub dev: String,
    /// The sender's clock, Unix milliseconds.
    pub ts: u64,
    /// `"text"` in version 1.
    pub kind: String,
}

/// The header version this build writes and reads.
pub const HEADER_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Frames (§8.2).
// ---------------------------------------------------------------------------

/// Client → gateway.
///
/// @description A frame the client sends on the social socket, one JSON
/// object per text frame, tagged by `t`: `hello` first, then `send`,
/// `global`, `ack` and `presence`.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Up {
    /// @description First, always: which of this account's chat devices
    /// this socket speaks for, and the status it starts in. Nothing else is
    /// accepted before it.
    Hello {
        /// @description The device id, as `POST /social/devices` registered
        /// it for this account.
        /// @internal
        /// The device id (§4.1), registered with `POST /social/devices`.
        device: String,
        /// @description The status to start in.
        presence: PresenceSet,
    },
    /// @description A sealed message to a conversation, answered `sent` or
    /// `refused` with the same `req`.
    /// @internal
    /// A sealed message to a conversation. Answered [`Down::Sent`] or
    /// [`Down::Refused`] with the same `req`.
    Send {
        /// @description The client's number for this send, echoed in the
        /// answer.
        req: u32,
        /// @description The conversation's id (`dm:<tag>:<tag>`,
        /// `grp:<uuid>`, `tbl:<game>` or `team:<game>:<n>`); it must equal
        /// the header's.
        /// @internal
        /// The conversation; must equal the header's.
        conv: String,
        /// @description The message, with its key wrapped once for every
        /// current device of every recipient (and optionally the sending
        /// device).
        /// @internal
        /// The message, with one wrap per recipient device.
        sealed: Sealed,
    },
    /// @description Every delivery up to and including `upto` is stored or
    /// shown: the gateway deletes the slots up to it that it sent this
    /// socket. A `seq` may arrive below one already seen, so de-duplicate by
    /// the message id, never by `seq`.
    /// @internal
    /// Every delivery up to and including `upto` is stored or shown; the
    /// gateway deletes the slots up to it that it sent this socket. A
    /// `seq` may arrive below one already seen (messages committed out of
    /// order): de-duplicate by the message id, never by `seq`.
    Ack {
        /// @description The highest `msg` `seq` the client has.
        /// @internal
        /// The highest [`Down::Msg`] `seq` the client has.
        upto: u64,
    },
    /// @description A new status or activity: `status` and `activity` sit
    /// beside `t`.
    /// @internal
    /// A new status or activity.
    Presence(PresenceSet),
    /// @description A public message to every online player in the lobby
    /// chat (`global`). It is not sealed: the gateway relays the body as it
    /// is to every other open social socket. Answered `sent` or `refused`
    /// with the same `req`.
    /// @internal
    /// A public message to all online players in the lobby chat (`Conv::Global`).
    Global {
        /// @description The client's number for this send, echoed in the
        /// answer.
        /// @internal
        /// The client's number for this send, echoed in [`Down::Sent`] or [`Down::Refused`].
        req: u32,
        /// @description The message's id, a version 7 UUID the sender makes.
        /// @internal
        /// The message's id, a `UUIDv7` the sender makes.
        id: String,
        /// @description The message body as JSON text, 1 to 4096 bytes.
        /// @internal
        /// The JSON-encoded message body, at most [`MAX_BODY_BYTES`].
        body: String,
    },
}

/// The status a client sets for itself.
///
/// @description The status a client sets for itself, and what it says it
/// is doing.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresenceSet {
    /// @description What friends are shown (`invisible` reads as
    /// `offline`).
    pub status: Status,
    /// @description What the client says it is doing, where the gateway
    /// cannot see it; absent otherwise.
    /// @internal
    /// What the client says it is doing, when the gateway cannot see it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity: Option<Activity>,
}

/// The status a client chooses.
///
/// @description The status a client chooses: `online`, `away` (the client
/// sets it after ten idle minutes), `dnd` (do not disturb) or `invisible`
/// (friends see `offline`).
/// @openapi
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Online.
    Online,
    /// Away (the client sets it after ten idle minutes).
    Away,
    /// Do not disturb.
    Dnd,
    /// Appear offline: friends see `offline`.
    Invisible,
}

/// What a client says it is doing, which the gateway cannot see itself.
///
/// @description What a client says it is doing, which the gateway cannot
/// see itself: `building` (in the deck builder) or `house` (playing the
/// house AI in a local game).
/// @openapi
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Activity {
    /// In the deck builder.
    Building,
    /// Playing the house AI in a local game.
    House,
}

/// A message as it is sent: the sealed body, and one wrap of its key per
/// recipient device. Every field is base64url without padding.
///
/// @description A message as it is sent: the sealed body, and one wrap of
/// its key per recipient device. Every value is base64url without padding;
/// the gateway checks shapes and sizes and cannot open it.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sealed {
    /// @description The header's JSON bytes (`v`, `id`, `conv`, `from`,
    /// `dev`, `ts`, `kind`) exactly as the sender signs them, at most 512
    /// bytes.
    /// @internal
    /// The header's JSON bytes ([`Header`]), at most [`MAX_HEADER_BYTES`].
    pub hdr: String,
    /// @description The body's nonce, 24 bytes.
    pub n: String,
    /// @description The body's ciphertext (XChaCha20-Poly1305 under the
    /// message key), at most 4112 bytes.
    /// @internal
    /// The body's ciphertext (XChaCha20-Poly1305 under the message key).
    pub ct: String,
    /// @description The message's ephemeral X25519 public key, 32 bytes.
    pub epk: String,
    /// @description The sending device's Ed25519 signature, 64 bytes.
    pub sig: String,
    /// @description One wrap per current device of every recipient,
    /// optionally one for the sending device too (to recover the message
    /// from its own inbox); no device twice.
    /// @internal
    /// One per recipient device, optionally including the sender for inbox recovery.
    pub wraps: Vec<Wrap>,
}

/// The message key wrapped for one device.
///
/// @description The message key wrapped for one device.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wrap {
    /// @description The device id: 16 lowercase hex characters.
    /// @internal
    /// The device id.
    pub d: String,
    /// @description The wrapped key, 48 bytes, base64url without padding.
    /// @internal
    /// The wrapped key, [`WRAP_BYTES`] bytes.
    pub k: String,
}

/// A message as one device receives it: [`Sealed`] with only that device's
/// wrap.
///
/// @description A sealed message as one device received it: the sender's
/// header, the sealed body and its signature, and the message key wrapped
/// for that device. Every field is base64url without padding; the gateway
/// cannot open it.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delivered {
    /// @description The header's JSON bytes exactly as the sender signed
    /// them, at most 512 bytes.
    /// @internal
    /// As [`Sealed::hdr`].
    pub hdr: String,
    /// @description The body's nonce, 24 bytes.
    /// @internal
    /// As [`Sealed::n`].
    pub n: String,
    /// @description The body's ciphertext (XChaCha20-Poly1305 under the
    /// message key), at most 4112 bytes.
    /// @internal
    /// As [`Sealed::ct`].
    pub ct: String,
    /// @description The message's ephemeral X25519 public key, 32 bytes.
    /// @internal
    /// As [`Sealed::epk`].
    pub epk: String,
    /// @description The sending device's Ed25519 signature, 64 bytes.
    /// @internal
    /// As [`Sealed::sig`].
    pub sig: String,
    /// @description The message key wrapped for this device, 48 bytes.
    /// @internal
    /// This device's wrap.
    pub k: String,
}

/// Gateway → client.
///
/// @description A frame the gateway sends on the social socket, one JSON
/// object per text frame, tagged by `t`.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Down {
    /// @description The answer to `hello`: who this socket is. A `snapshot`
    /// follows.
    /// @internal
    /// The answer to [`Up::Hello`].
    Welcome {
        /// @description This account's tag.
        tag: String,
        /// @description This account's handle (`Alice#af03`).
        handle: String,
        /// @description The device the socket speaks for.
        device: String,
        /// @description Whether the account is a guest.
        guest: bool,
    },
    /// @description The whole social state, its fields beside `t`: sent
    /// after `welcome`, and again after any change the gateway cannot say
    /// as one event.
    /// @internal
    /// The whole social state; after `Welcome`, and after any change the
    /// gateway cannot say as one event.
    Snapshot(Snapshot),
    /// @description A delivery, carrying only this device's wrap; acked
    /// with `ack`. Sent again on the next connection until acked.
    /// @internal
    /// A delivery, acked with [`Up::Ack`].
    Msg {
        /// @description The slot's number, rising per device; slots may
        /// arrive out of order.
        /// @internal
        /// The slot's number, rising per device.
        seq: u64,
        /// @description When the gateway received it, Unix milliseconds:
        /// the order to show it in.
        /// @internal
        /// When the gateway received it, Unix milliseconds: the order.
        at: u64,
        /// @description The conversation's id.
        /// @internal
        /// The conversation.
        conv: String,
        /// @description Who sent it, as the gateway attests; check the
        /// header's `from` and `dev` against it.
        /// @internal
        /// Who sent it, as the gateway attests.
        from: Sender,
        /// @description The message.
        sealed: Delivered,
    },
    /// @description A public message in the lobby chat (`global`), relayed
    /// as it was sent, not sealed. After `snapshot` the gateway sends the
    /// last 50 it holds in memory.
    /// @internal
    /// A public message in the global lobby chat (`Conv::Global`).
    Global {
        /// @description The message's id, a version 7 UUID.
        /// @internal
        /// The message's id (`UUIDv7`).
        id: String,
        /// @description When the gateway received it, Unix milliseconds.
        at: u64,
        /// @description Who sent it, as the gateway attests.
        from: Sender,
        /// @description The message body as JSON text.
        /// @internal
        /// The JSON-encoded message body.
        body: String,
    },
    /// @description A `send` or `global` was taken.
    /// @internal
    /// A send was stored.
    Sent {
        /// @description The send's `req`.
        req: u32,
        /// @description The message's id.
        id: String,
        /// @description When the gateway received it, Unix milliseconds.
        at: u64,
    },
    /// @description A send, a hello or the socket was refused. A refused
    /// `hello` (`device`) or socket (`protocol`) is the last frame before
    /// the socket closes.
    /// @internal
    /// A send, a hello or the socket was refused.
    Refused {
        /// @description The send's `req`, when a send was refused.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        req: Option<u32>,
        /// @description Why.
        why: Why,
        /// @description With `devices`: the device ids the wraps left out.
        /// @internal
        /// With [`Why::Devices`]: device ids the wraps left out.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        missing: Vec<String>,
        /// @description With `devices`: the device ids the wraps named that
        /// are not recipients.
        /// @internal
        /// With [`Why::Devices`]: device ids the wraps named that are not
        /// recipients.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        extra: Vec<String>,
        /// @description A sentence for a person, when there is one (a
        /// version refusal names both versions).
        /// @internal
        /// A sentence for a person, when there is one (a version refusal).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    /// @description A friend's presence changed; pushed at most once a
    /// second per account.
    /// @internal
    /// A friend's presence changed.
    Presence {
        /// @description The friend's tag.
        tag: String,
        /// @description What it is now.
        presence: Presence,
    },
    /// @description A friend, request or block changed.
    Friend {
        /// @description What happened.
        event: FriendEvent,
        /// @description To whom.
        person: Person,
        /// @description Their presence, when they are now a friend.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        presence: Option<Presence>,
    },
    /// @description A group this account is in, or was invited to,
    /// changed.
    Group {
        /// @description What happened.
        event: GroupEvent,
        /// @description The group as it is now.
        group: GroupInfo,
        /// @description Who invited this account, on `invited`, so its
        /// client can list the invite at once; absent on every other event,
        /// and from a gateway older than this field, whose client lists the
        /// invite from the next `snapshot`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        inviter: Option<Person>,
    },
    /// @description The members of a table or team conversation, whenever
    /// they change (and to an account that left it).
    /// @internal
    /// The members of a table or team conversation, whenever they change.
    Members {
        /// @description The conversation's id.
        /// @internal
        /// The conversation.
        conv: String,
        /// @description Its members now.
        members: Vec<Person>,
    },
    /// @description The devices of `tag` changed: fetch its keys again
    /// (`GET /social/keys`).
    /// @internal
    /// The devices of `tag` changed: fetch its keys again.
    KeysChanged {
        /// @description Whose devices.
        /// @internal
        /// Whose.
        tag: String,
    },
    /// @description An invite to a table, its fields beside `t`.
    /// @internal
    /// An invite to a table.
    Invite(TableInvite),
    /// @description A table invite is no longer open.
    /// @internal
    /// An invite is no longer open.
    InviteGone {
        /// @description The invite's id.
        id: String,
        /// @description How it was answered; absent when it lapsed or was
        /// withdrawn unanswered.
        /// @internal
        /// How it was answered; `None` when it lapsed or was withdrawn
        /// unanswered.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        answer: Option<InviteAnswer>,
    },
    /// @description A line the gateway writes into a conversation, drawn as
    /// a server notice.
    /// @internal
    /// A line the gateway writes into a conversation (drawn as a server
    /// notice).
    Notice {
        /// @description The conversation's id.
        /// @internal
        /// The conversation.
        conv: String,
        /// @description What it says.
        notice: Notice,
    },
}

/// An account as another one sees it.
///
/// @description An account as another one sees it.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Person {
    /// @description The account's tag.
    /// @internal
    /// The tag.
    pub tag: String,
    /// @description Its handle (`Alice#af03`).
    /// @internal
    /// The handle (`Alice#af03`).
    pub handle: String,
}

/// Who sent a delivered message, as the gateway attests it.
///
/// @description Who sent a message, as the gateway attests it.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sender {
    /// @description The sender's tag; the header's `from` must equal it.
    pub tag: String,
    /// @description The sender's handle.
    pub handle: String,
    /// @description The sending device; the header's `dev` must equal it.
    pub device: String,
}

/// A friend's presence, as shown to friends only.
///
/// @description A friend's presence, as shown to friends only.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Presence {
    /// @description Their status; `invisible` is shown as `offline`.
    pub status: Shown,
    /// @description Where they are, when they are not offline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<Place>,
}

impl Presence {
    /// Offline, nowhere.
    #[must_use]
    pub const fn offline() -> Self {
        Self {
            status: Shown::Offline,
            place: None,
        }
    }
}

/// A status as a friend is shown it.
///
/// @description A status as a friend is shown it: `offline` (no social
/// socket open, or appearing offline), `online`, `away` or `dnd`.
/// @openapi
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shown {
    /// No social socket open, or appearing offline.
    Offline,
    /// Online.
    Online,
    /// Away.
    Away,
    /// Do not disturb.
    Dnd,
}

/// Where a friend is (§6). `game` is there only when they share it and the
/// room is not locked.
///
/// @description Where a friend is, tagged by `kind`. `game` is there only
/// when they share their table and it has no password.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Place {
    /// @description In the lobby.
    Lobby,
    /// @description In a waiting room.
    Room {
        /// @description The lobby game, when shared.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        game: Option<String>,
        /// @description The room's name.
        name: String,
        /// @description Its chairs.
        seats: u8,
        /// @description How many of its chairs are taken.
        /// @internal
        /// Its chairs taken.
        taken: u8,
        /// @description Whether it has a password.
        locked: bool,
    },
    /// @description Seated at a running table.
    Playing {
        /// @description The lobby game, when shared.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        game: Option<String>,
        /// @description The table's name.
        name: String,
        /// @description Its chairs.
        seats: u8,
        /// @description How long it has been running, in seconds.
        running_secs: u32,
    },
    /// @description Watching a table.
    Watching {
        /// @description The lobby game, when shared.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        game: Option<String>,
        /// @description The table's name.
        name: String,
    },
    /// @description In the deck builder (the client says so).
    Building,
    /// @description Playing the house AI in a local game (the client says
    /// so).
    House,
}

/// The whole social state of one account, as its device is sent it.
///
/// @description The whole social state of one account, as its device is
/// sent it.
/// @openapi
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    /// @description Accepted friends, with their presence.
    /// @internal
    /// Accepted friends.
    pub friends: Vec<Friend>,
    /// @description Friend requests to this account.
    /// @internal
    /// Requests to this account.
    pub incoming: Vec<Person>,
    /// @description Friend requests from this account.
    /// @internal
    /// Requests from this account.
    pub outgoing: Vec<Person>,
    /// @description Accounts this account blocked.
    pub blocked: Vec<Person>,
    /// @description Groups this account is in.
    pub groups: Vec<GroupInfo>,
    /// @description Groups this account is invited to.
    pub group_invites: Vec<GroupInvite>,
    /// @description The table and team conversations this account is in
    /// now.
    /// @internal
    /// Table and team conversations this account is in now.
    pub tables: Vec<TableConv>,
    /// @description Open table invites to this account.
    pub invites: Vec<TableInvite>,
    /// @description The account's social settings.
    pub settings: SocialSettings,
}

/// A friend.
///
/// @description A friend.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Friend {
    /// @description Who.
    pub person: Person,
    /// @description Friends since when, Unix milliseconds.
    /// @internal
    /// Since when, Unix milliseconds.
    pub since: u64,
    /// @description Their presence now.
    pub presence: Presence,
}

/// A group, with its members and pending invites.
///
/// @description A group, with its members and pending invites.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupInfo {
    /// @description The group's id (a UUID); its conversation is
    /// `grp:<id>`.
    pub id: String,
    /// @description Its name.
    pub name: String,
    /// @description Its members, in the order they joined.
    pub members: Vec<Member>,
    /// @description Accounts invited and not yet answered.
    pub invited: Vec<Person>,
}

/// A member of a group.
///
/// @description A member of a group.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Member {
    /// @description Who.
    pub person: Person,
    /// @description Whether they own the group.
    pub owner: bool,
}

/// An invite to a group, as the invitee sees it.
///
/// @description An invite to a group, as the invitee sees it.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupInvite {
    /// @description The group's id.
    pub group: String,
    /// @description Its name.
    pub name: String,
    /// @description Who invited.
    pub inviter: Person,
    /// @description When, Unix milliseconds.
    pub at: u64,
}

/// A table or team conversation this account is in.
///
/// @description A table or team conversation this account is in.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableConv {
    /// @description The conversation's id: `tbl:<game>` or
    /// `team:<game>:<n>`.
    /// @internal
    /// `tbl:<game>` or `team:<game>:<n>`.
    pub conv: String,
    /// @description The table's name.
    pub name: String,
    /// @description Its members now.
    pub members: Vec<Person>,
}

/// An invite to sit at, or watch, a table.
///
/// @description An invite to sit at, or watch, a table. It lives ten
/// minutes.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableInvite {
    /// @description The invite's id; `POST /lobby/games/{id}/join` takes
    /// it as `invite`.
    /// @internal
    /// The invite's id; `POST /lobby/games/{game}/join` takes it as `invite`.
    pub id: String,
    /// @description Who invited.
    pub from: Person,
    /// @description The lobby game.
    pub game: String,
    /// @description Join a waiting room, or watch a running table.
    pub kind: InviteKind,
    /// @description The table as it was when invited.
    pub table: TableBrief,
    /// @description Seconds until it lapses.
    pub expires_in: u32,
}

/// What a table invite is for.
///
/// @description What a table invite is for: `join` (sit down in a waiting
/// room) or `watch` (watch a running table).
/// @openapi
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InviteKind {
    /// Sit down in a waiting room.
    Join,
    /// Watch a running table.
    Watch,
}

/// How a table invite was answered.
///
/// @description How a table invite was answered: `accepted` (the invitee
/// joined with it) or `declined`.
/// @openapi
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InviteAnswer {
    /// The invitee joined with it.
    Accepted,
    /// The invitee said no.
    Declined,
}

/// A table, briefly.
///
/// @description A table, briefly.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableBrief {
    /// @description Its name.
    pub name: String,
    /// @description Its chairs.
    pub seats: u8,
    /// @description How many of its chairs are taken.
    /// @internal
    /// Its chairs taken.
    pub taken: u8,
    /// @description Where it stands: `waiting`, `playing` or `over`.
    /// @openapi enum: [waiting, playing, over]
    /// @internal
    /// `waiting`, `playing` or `over`.
    pub state: String,
    /// @description Whether it has a password.
    pub locked: bool,
}

/// The settings the gateway enforces for an account.
/// @openapi
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialSettings {
    /// @description Who may whisper this account.
    pub whispers_from: WhispersFrom,
    /// @description Whether others may send this account a friend request.
    pub requests_open: bool,
    /// @description Whether friends are shown which table this account is
    /// at (never a table with a password).
    pub share_table: bool,
}

impl Default for SocialSettings {
    fn default() -> Self {
        Self {
            whispers_from: WhispersFrom::FriendsAndTables,
            requests_open: true,
            share_table: true,
        }
    }
}

/// Who may whisper an account (§5.2).
///
/// @description Who may whisper an account: `friends`, or
/// `friends_and_tables` (also the players with a chair at a table it sits
/// at; the default).
/// @openapi
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WhispersFrom {
    /// Friends only.
    Friends,
    /// Friends, and the players at this account's table (the default).
    #[default]
    FriendsAndTables,
}

impl WhispersFrom {
    /// The database's number for it (`account.whispers_from`).
    #[must_use]
    pub const fn code(self) -> i16 {
        match self {
            Self::Friends => 0,
            Self::FriendsAndTables => 1,
        }
    }

    /// The setting a database number stands for; anything unknown is the
    /// stricter one.
    #[must_use]
    pub const fn from_code(code: i16) -> Self {
        match code {
            1 => Self::FriendsAndTables,
            _ => Self::Friends,
        }
    }
}

/// What happened to a friend, a request or a block.
///
/// @description What happened to a friend, a request or a block:
/// `request_in` (they asked to be friends), `request_out` (this account
/// asked them, from another of its devices), `accepted` (a request was
/// accepted, either way), `removed` (the friendship ended), `request_gone`
/// (a request was declined, cancelled or withdrawn), `blocked` or
/// `unblocked` (by this account).
/// @openapi
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FriendEvent {
    /// They asked to be friends.
    RequestIn,
    /// This account asked them (from another of its devices).
    RequestOut,
    /// A request was accepted, either way.
    Accepted,
    /// The friendship ended.
    Removed,
    /// A request was declined, cancelled or withdrawn.
    RequestGone,
    /// This account blocked them.
    Blocked,
    /// This account unblocked them.
    Unblocked,
}

/// What happened to a group.
///
/// @description What happened to a group: `joined` (this account is now a
/// member), `updated` (its name, members, owner or invites changed), `left`
/// (this account left or was removed), `invited` (this account was
/// invited), `invite_gone` (this account's invite is no longer open) or
/// `deleted`.
/// @openapi
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupEvent {
    /// This account is now a member.
    Joined,
    /// Its name, members, owner or invites changed.
    Updated,
    /// This account left or was removed.
    Left,
    /// This account was invited.
    Invited,
    /// This account's invite is no longer open.
    InviteGone,
    /// The group was deleted.
    Deleted,
}

/// A line the gateway writes into a conversation.
///
/// @description A line the gateway writes into a group's conversation,
/// tagged by `kind`.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Notice {
    /// @description Someone joined the group.
    MemberJoined {
        /// @description Who.
        person: Person,
    },
    /// @description Someone left the group or was removed.
    MemberLeft {
        /// @description Who.
        person: Person,
    },
    /// @description The group was renamed.
    Renamed {
        /// @description Its new name.
        name: String,
    },
    /// @description The group has a new owner.
    OwnerNow {
        /// @description Who.
        person: Person,
    },
}

/// Why something was refused.
///
/// @description Why something was refused: `protocol` (the socket states
/// another protocol or chat version; it closes), `device` (the hello names
/// a device that is not this account's; register again), `not_member` (the
/// sender may not write to this conversation, for whatever reason),
/// `devices` (the wraps do not name exactly the recipients' devices; see
/// `missing` and `extra`), `rate` (over the send allowance; retry in a
/// second), `too_large`, `bad_sealed` (malformed: base64, sizes, the
/// header), `stale` (the header's `ts` is more than ten minutes from the
/// gateway's clock), `duplicate` (a message with this id is stored
/// already), `recipient_full` (a recipient device holds 5000 undelivered
/// messages) or `off` (this gateway has no chat).
/// @openapi
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Why {
    /// The socket states another protocol or social version; it closes.
    Protocol,
    /// The hello names a device that is not this account's; register again.
    Device,
    /// The sender may not write to this conversation (also: blocked, not
    /// friends, whispers not allowed; the sender cannot tell which).
    NotMember,
    /// The wraps do not name exactly the recipients' devices
    /// (`missing`/`extra`).
    Devices,
    /// Over the send allowance; retry in a second.
    Rate,
    /// The frame or a field is too large.
    TooLarge,
    /// The sealed message is malformed (bad base64, sizes, header).
    BadSealed,
    /// The header's `ts` is more than ten minutes from the gateway's clock.
    Stale,
    /// A message with this id was already stored.
    Duplicate,
    /// @description A recipient device holds 5000 undelivered messages.
    /// @internal
    /// A recipient device holds [`SLOTS_PER_DEVICE`] undelivered messages.
    RecipientFull,
    /// This gateway has no chat.
    Off,
}

// ---------------------------------------------------------------------------
// HTTP bodies (§8.3), shared so neither end spells them twice.
// ---------------------------------------------------------------------------

/// `POST /social/devices`.
///
/// @description A chat device to register: its two public keys, the
/// identity key's statement over them for the account's tag, and its OS
/// family.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceRegistration {
    /// @description The device's Ed25519 identity key, 32 bytes, base64url
    /// without padding.
    pub ik: String,
    /// @description The device's X25519 key, 32 bytes, base64url without
    /// padding.
    pub dk: String,
    /// @description The identity key's signature over the device statement
    /// for the account's tag, 64 bytes, base64url without padding.
    /// @internal
    /// The statement's signature (§4.1), 64 bytes, base64url.
    pub dsig: String,
    /// @description The device's OS family (`macOS`), 1 to 24 characters,
    /// none of them a control, bidirectional or invisible character.
    /// @internal
    /// The OS family (`macOS`), at most [`MAX_DEVICE_LABEL`] characters.
    pub label: String,
}

/// The answer to `POST /social/devices` (`201`), and to anything else that
/// creates something with an id.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Created {
    /// @description The new thing's id: a device id, a group's UUID or a
    /// table invite's id.
    pub id: String,
}

/// One of this account's devices, as `GET /social/devices` lists it.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceRow {
    /// @description The device id, its fingerprint: 16 lowercase hex
    /// characters derived from its identity key.
    /// @internal
    /// The device id (its fingerprint).
    pub id: String,
    /// @description The device's OS family.
    pub label: String,
    /// @description When it was registered, Unix milliseconds.
    pub added_at: u64,
    /// @description When it last opened the social socket, Unix
    /// milliseconds.
    pub seen_at: u64,
}

/// `GET /social/devices`.
///
/// @description The account's chat devices.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceList {
    /// @description This account's devices, oldest first.
    pub devices: Vec<DeviceRow>,
}

/// One device's public keys, as `GET /social/keys` gives them.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceKey {
    /// @description The device id.
    pub id: String,
    /// @description Its Ed25519 identity key, 32 bytes, base64url without
    /// padding.
    pub ik: String,
    /// @description Its X25519 key, 32 bytes, base64url without padding.
    pub dk: String,
    /// @description The identity key's signature over the device statement,
    /// base64url without padding: check it against the account's tag
    /// before trusting the keys.
    /// @internal
    /// The statement's signature, base64url: check it (§4.1).
    pub dsig: String,
    /// @description Its OS family.
    pub label: String,
    /// @description When it was registered, Unix milliseconds.
    pub added_at: u64,
}

/// One account's devices.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountKeys {
    /// @description The account's tag.
    pub tag: String,
    /// @description Its handle (`Alice#af03`).
    pub handle: String,
    /// @description Its devices.
    pub devices: Vec<DeviceKey>,
}

/// `GET /social/keys?tag=…`: only the tags this account may write to; any
/// other tag is left out rather than refused.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeysAnswer {
    /// @description One entry per tag asked for that the caller may write
    /// to.
    pub keys: Vec<AccountKeys>,
}

/// `GET /social/players/{handle}`.
///
/// @description A player as the caller sees them, and how the two stand.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct PlayerLookup {
    /// @description Their tag.
    pub tag: String,
    /// @description Their handle (`Alice#af03`).
    pub handle: String,
    /// @description Whether they are the caller's friend.
    pub friend: bool,
    /// @description Whether they asked the caller to be friends.
    pub incoming: bool,
    /// @description Whether the caller asked them.
    pub outgoing: bool,
    /// @description Whether the caller blocked them.
    pub blocked: bool,
}

/// `POST /social/friends/requests`.
///
/// @description A friend request.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriendRequest {
    /// @description Whom to ask, by handle: `Name#tag`.
    pub handle: String,
}

/// The answer to a friend request (`201`).
/// @openapi
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestAnswer {
    /// @description `pending`, or `friends` when they had asked already.
    pub state: RequestState,
}

/// Where a friend request stands.
///
/// @description Where a friend request stands: `pending` (sent,
/// unanswered) or `friends` (they had asked already).
/// @openapi
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestState {
    /// Sent, unanswered.
    Pending,
    /// They had asked already: now friends.
    Friends,
}

/// A body that names one account by tag (`POST /social/blocks`,
/// `POST /social/groups/{id}/invites`, `…/owner`).
///
/// @description One account, by tag.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagBody {
    /// @description The account's tag: lowercase hex, four to eight digits.
    pub tag: String,
}

/// A body that names a group (`POST /social/groups`, `PATCH …/{id}`).
///
/// @description A group's name.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupName {
    /// @description The name: 1 to 48 characters, none of them hidden.
    /// @internal
    /// The name ([`crate::names::group_name`]).
    pub name: String,
}

/// `POST /social/invites`.
///
/// @description An invite to the caller's table.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InviteRequest {
    /// @description The friend to invite, by tag.
    pub tag: String,
    /// @description The lobby game the caller sits in.
    pub game: String,
}

/// `DELETE /social/invites/{id}`'s optional body.
///
/// @description What the invitee says when it closes an invite.
/// @openapi
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InviteDecline {
    /// @description `declined`; anything else, or no body, lets the invite
    /// go unanswered.
    /// @internal
    /// `declined`.
    pub answer: InviteAnswer,
}

/// `POST /social/reports`: one delivered message, revealed (§4.7).
///
/// @description One delivered message, revealed by its recipient: the
/// message as it arrived, its message key and the body it decrypted to.
/// Revealing the key reveals this one message and nothing else.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatReport {
    /// @description The conversation it was delivered in, by id
    /// (`dm:<tag>:<tag>`, `grp:<uuid>`, `tbl:<game>`, `team:<game>:<n>` or
    /// `global`); the message's header must name the same.
    /// @internal
    /// The conversation it was delivered in.
    pub conv: String,
    /// @description The message as it was delivered.
    pub sealed: Delivered,
    /// @description Its message key, 32 bytes, base64url without padding.
    pub mk: String,
    /// @description The body it decrypted to, the exact bytes (at most
    /// 4096), base64url without padding.
    /// @internal
    /// Its decrypted body, base64url of the exact bytes.
    pub body: String,
    /// @description What the reporter says, at most 2000 characters; empty
    /// when left out.
    /// @internal
    /// What the reporter says, at most [`MAX_REPORT_NOTE`] characters.
    #[serde(default)]
    pub note: String,
}

/// The answer to `POST /social/reports` (`201`).
///
/// @description What a report route answers when the feedback service took
/// the report (`POST /social/reports`, and `POST /reports` in the same
/// shape).
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportReceipt {
    /// @description The feedback service's id for the report.
    pub report_id: String,
}

/// `GET /info`'s `social`, or `null` when the gateway has no chat.
/// @openapi
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialInfo {
    /// @description The chat protocol the social socket speaks.
    /// @internal
    /// [`SOCIAL_VERSION`].
    pub version: u32,
    /// @description How many days an undelivered message is kept for a
    /// registered account's device (`BAYLEE_SOCIAL_KEEP_DAYS`, 1 to 30); a
    /// guest's device keeps it one day.
    /// @internal
    /// How many days undelivered messages are kept
    /// (`BAYLEE_SOCIAL_KEEP_DAYS`); the notice says it.
    pub keep_days: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_socket_path_states_both_versions_and_no_token() {
        let path = social_socket_path("tick");
        assert_eq!(
            path,
            format!(
                "/social/ws?ticket=tick&protocol={}&social={SOCIAL_VERSION}",
                crate::PROTOCOL_VERSION
            )
        );
        assert!(!path.contains("token="));
        assert!(social_version_refusal("This client", SOCIAL_VERSION).is_none());
        let said = social_version_refusal("This client", 9).expect("refused");
        assert!(said.contains('9') && said.contains(&SOCIAL_VERSION.to_string()));
    }

    #[test]
    fn a_whisper_names_its_tags_in_numeric_order_whichever_way_it_is_built() {
        let one = Conv::dm("af03", "0012").expect("two tags");
        let two = Conv::dm("0012", "af03").expect("two tags");
        assert_eq!(one, two);
        assert_eq!(one.to_string(), "dm:0012:af03");
        // Length first: a five-digit tag is greater than any four-digit one.
        assert_eq!(
            Conv::dm("10000", "ffff").expect("tags").to_string(),
            "dm:ffff:10000"
        );
        assert_eq!(one.partner("af03"), Some("0012"));
        assert_eq!(one.partner("0012"), Some("af03"));
        assert_eq!(one.partner("beef"), None);
        assert_eq!(Conv::dm("af03", "af03"), None);
        assert_eq!(Conv::dm("AF03", "0012"), None);
    }

    #[test]
    fn every_conversation_reads_back_what_it_writes_and_nothing_else() {
        for text in [
            "dm:0012:af03",
            "grp:01923f3c-7a2b-7c00-8000-0123456789ab",
            "tbl:g-12_ab",
            "team:g-12:1",
        ] {
            let conv: Conv = text.parse().expect(text);
            assert_eq!(conv.to_string(), text);
            let json = serde_json::to_string(&conv).expect("serializes");
            assert_eq!(json, format!("\"{text}\""));
            assert_eq!(serde_json::from_str::<Conv>(&json).expect("reads"), conv);
        }
        for bad in [
            "dm:af03:0012",
            "dm:af03:af03",
            "dm:0012",
            "dm:0012:AF03",
            "dm:0012:00af03",
            "grp:not-a-uuid",
            "grp:01923F3C-7A2B-7C00-8000-0123456789AB",
            "tbl:",
            "tbl:a:b",
            "team:g:01",
            "team:g:256",
            "team:g",
            "chat:x",
            "",
        ] {
            assert!(bad.parse::<Conv>().is_err(), "{bad}");
        }
    }

    #[test]
    fn frames_are_tagged_by_t_in_snake_case() {
        let hello = Up::Hello {
            device: "0123456789abcdef".into(),
            presence: PresenceSet {
                status: Status::Invisible,
                activity: None,
            },
        };
        let json = serde_json::to_value(&hello).expect("serializes");
        assert_eq!(
            json,
            serde_json::json!({"t":"hello","device":"0123456789abcdef","presence":{"status":"invisible"}})
        );
        let refused = Down::Refused {
            req: None,
            why: Why::Protocol,
            missing: vec![],
            extra: vec![],
            message: Some("old".into()),
        };
        assert_eq!(
            serde_json::to_value(&refused).expect("serializes"),
            serde_json::json!({"t":"refused","why":"protocol","message":"old"})
        );
        let presence = Up::Presence(PresenceSet {
            status: Status::Away,
            activity: Some(Activity::Building),
        });
        let back: Up =
            serde_json::from_str(&serde_json::to_string(&presence).expect("ser")).expect("de");
        assert_eq!(back, presence);
        let place = Place::Room {
            game: None,
            name: "Kitchen".into(),
            seats: 4,
            taken: 2,
            locked: true,
        };
        assert_eq!(
            serde_json::to_value(&place).expect("ser"),
            serde_json::json!({"kind":"room","name":"Kitchen","seats":4,"taken":2,"locked":true})
        );
        let notice = Down::Notice {
            conv: "grp:x".into(),
            notice: Notice::Renamed {
                name: "Crew".into(),
            },
        };
        assert_eq!(
            serde_json::to_value(&notice).expect("ser"),
            serde_json::json!({"t":"notice","conv":"grp:x","notice":{"kind":"renamed","name":"Crew"}})
        );
    }

    #[test]
    fn the_whisper_setting_has_a_stable_database_number() {
        for w in [WhispersFrom::Friends, WhispersFrom::FriendsAndTables] {
            assert_eq!(WhispersFrom::from_code(w.code()), w);
        }
        assert_eq!(WhispersFrom::from_code(7), WhispersFrom::Friends);
        assert_eq!(
            serde_json::to_value(SocialSettings::default()).expect("ser"),
            serde_json::json!({"whispers_from":"friends_and_tables","requests_open":true,"share_table":true})
        );
    }

    #[test]
    fn shapes_of_tags_devices_and_ids() {
        assert!(valid_tag("0001") && valid_tag("af03") && valid_tag("10000"));
        assert!(
            !valid_tag("001") && !valid_tag("00af3") && !valid_tag("AF03") && !valid_tag("af0g")
        );
        assert!(valid_device_id("0123456789abcdef"));
        assert!(!valid_device_id("0123456789ABCDEF") && !valid_device_id("0123"));
        assert!(valid_uuid("01923f3c-7a2b-7c00-8000-0123456789ab"));
        assert!(!valid_uuid("01923f3c7a2b7c0080000123456789ab0000"));
    }

    #[test]
    fn the_worst_case_send_fits_a_frame() {
        // 50 members × 5 devices, including optional sender recovery: 250 wraps of 48 bytes.
        let wrap = Wrap {
            d: "0123456789abcdef".into(),
            k: "A".repeat(64),
        };
        let sealed = Sealed {
            hdr: "A".repeat(MAX_HEADER_BYTES.div_ceil(3) * 4),
            n: "A".repeat(32),
            ct: "A".repeat((MAX_BODY_BYTES + 16).div_ceil(3) * 4),
            epk: "A".repeat(43),
            sig: "A".repeat(86),
            wraps: vec![wrap; MAX_GROUP_MEMBERS * MAX_DEVICES],
        };
        let frame = Up::Send {
            req: u32::MAX,
            conv: "grp:01923f3c-7a2b-7c00-8000-0123456789ab".into(),
            sealed,
        };
        let size = serde_json::to_vec(&frame).expect("ser").len();
        assert!(size < MAX_SOCIAL_FRAME, "{size}");
    }
}
