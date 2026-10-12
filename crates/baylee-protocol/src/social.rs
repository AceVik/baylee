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
        }
    }
}

impl FromStr for Conv {
    type Err = ConvFault;

    /// Exactly the canonical text [`Display`](fmt::Display) writes: a
    /// whisper with its tags out of order, or a team number with a leading
    /// zero, is refused rather than read as another spelling of one id.
    fn from_str(text: &str) -> Result<Self, ConvFault> {
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Up {
    /// First, always: which of this account's devices this is, and the
    /// status it starts in. Nothing else is accepted before it.
    Hello {
        /// The device id (§4.1), registered with `POST /social/devices`.
        device: String,
        /// The status to start in.
        presence: PresenceSet,
    },
    /// A sealed message to a conversation. Answered [`Down::Sent`] or
    /// [`Down::Refused`] with the same `req`.
    Send {
        /// The client's number for this send, echoed in the answer.
        req: u32,
        /// The conversation; must equal the header's.
        conv: String,
        /// The message, with one wrap per recipient device.
        sealed: Sealed,
    },
    /// Every delivery up to and including `upto` is stored or shown; the
    /// gateway deletes their slots.
    Ack {
        /// The highest [`Down::Msg`] `seq` the client has.
        upto: u64,
    },
    /// A new status or activity.
    Presence(PresenceSet),
}

/// The status a client sets for itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresenceSet {
    /// What friends are shown (`invisible` reads as offline).
    pub status: Status,
    /// What the client says it is doing, when the gateway cannot see it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity: Option<Activity>,
}

/// The status a client chooses.
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sealed {
    /// The header's JSON bytes ([`Header`]), at most [`MAX_HEADER_BYTES`].
    pub hdr: String,
    /// The body's nonce, 24 bytes.
    pub n: String,
    /// The body's ciphertext (XChaCha20-Poly1305 under the message key).
    pub ct: String,
    /// The message's ephemeral X25519 public key, 32 bytes.
    pub epk: String,
    /// The sending device's Ed25519 signature, 64 bytes.
    pub sig: String,
    /// One per recipient device, the sending device excepted.
    pub wraps: Vec<Wrap>,
}

/// The message key wrapped for one device.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wrap {
    /// The device id.
    pub d: String,
    /// The wrapped key, [`WRAP_BYTES`] bytes.
    pub k: String,
}

/// A message as one device receives it: [`Sealed`] with only that device's
/// wrap.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delivered {
    /// As [`Sealed::hdr`].
    pub hdr: String,
    /// As [`Sealed::n`].
    pub n: String,
    /// As [`Sealed::ct`].
    pub ct: String,
    /// As [`Sealed::epk`].
    pub epk: String,
    /// As [`Sealed::sig`].
    pub sig: String,
    /// This device's wrap.
    pub k: String,
}

/// Gateway → client.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Down {
    /// The answer to [`Up::Hello`].
    Welcome {
        /// This account's tag.
        tag: String,
        /// This account's handle (`Alice#af03`).
        handle: String,
        /// The device the socket speaks for.
        device: String,
        /// Whether the account is a guest.
        guest: bool,
    },
    /// The whole social state; after `Welcome`, and after any change the
    /// gateway cannot say as one event.
    Snapshot(Snapshot),
    /// A delivery, acked with [`Up::Ack`].
    Msg {
        /// The slot's number, rising per device.
        seq: u64,
        /// When the gateway received it, Unix milliseconds: the order.
        at: u64,
        /// The conversation.
        conv: String,
        /// Who sent it, as the gateway attests.
        from: Sender,
        /// The message.
        sealed: Delivered,
    },
    /// A send was stored.
    Sent {
        /// The send's `req`.
        req: u32,
        /// The message's id.
        id: String,
        /// When the gateway received it, Unix milliseconds.
        at: u64,
    },
    /// A send, a hello or the socket was refused.
    Refused {
        /// The send's `req`, when a send was refused.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        req: Option<u32>,
        /// Why.
        why: Why,
        /// With [`Why::Devices`]: device ids the wraps left out.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        missing: Vec<String>,
        /// With [`Why::Devices`]: device ids the wraps named that are not
        /// recipients.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        extra: Vec<String>,
        /// A sentence for a person, when there is one (a version refusal).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    /// A friend's presence changed.
    Presence {
        /// The friend's tag.
        tag: String,
        /// What it is now.
        presence: Presence,
    },
    /// A friend, request or block changed.
    Friend {
        /// What happened.
        event: FriendEvent,
        /// To whom.
        person: Person,
        /// Their presence, when they are now a friend.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        presence: Option<Presence>,
    },
    /// A group this account is in, or was invited to, changed.
    Group {
        /// What happened.
        event: GroupEvent,
        /// The group as it is now.
        group: GroupInfo,
    },
    /// The members of a table or team conversation, whenever they change.
    Members {
        /// The conversation.
        conv: String,
        /// Its members now.
        members: Vec<Person>,
    },
    /// The devices of `tag` changed: fetch its keys again.
    KeysChanged {
        /// Whose.
        tag: String,
    },
    /// An invite to a table.
    Invite(TableInvite),
    /// An invite is no longer open.
    InviteGone {
        /// The invite's id.
        id: String,
        /// How it was answered; `None` when it lapsed or was withdrawn
        /// unanswered.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        answer: Option<InviteAnswer>,
    },
    /// A line the gateway writes into a conversation (drawn as a server
    /// notice).
    Notice {
        /// The conversation.
        conv: String,
        /// What it says.
        notice: Notice,
    },
}

/// An account as another one sees it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Person {
    /// The tag.
    pub tag: String,
    /// The handle (`Alice#af03`).
    pub handle: String,
}

/// Who sent a delivered message, as the gateway attests it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sender {
    /// The sender's tag; the header's `from` must equal it.
    pub tag: String,
    /// The sender's handle.
    pub handle: String,
    /// The sending device; the header's `dev` must equal it.
    pub device: String,
}

/// A friend's presence, as shown to friends only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Presence {
    /// Their status; `invisible` is shown as `offline`.
    pub status: Shown,
    /// Where they are, when they are not offline.
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Place {
    /// In the lobby.
    Lobby,
    /// In a waiting room.
    Room {
        /// The lobby game, when shared.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        game: Option<String>,
        /// The room's name.
        name: String,
        /// Its chairs.
        seats: u8,
        /// Its chairs taken.
        taken: u8,
        /// Whether it has a password.
        locked: bool,
    },
    /// Seated at a running table.
    Playing {
        /// The lobby game, when shared.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        game: Option<String>,
        /// The table's name.
        name: String,
        /// Its chairs.
        seats: u8,
        /// How long it has been running, in seconds.
        running_secs: u32,
    },
    /// Watching a table.
    Watching {
        /// The lobby game, when shared.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        game: Option<String>,
        /// The table's name.
        name: String,
    },
    /// In the deck builder (the client says so).
    Building,
    /// Playing the house AI in a local game (the client says so).
    House,
}

/// The whole social state of one account, as its device is sent it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    /// Accepted friends.
    pub friends: Vec<Friend>,
    /// Requests to this account.
    pub incoming: Vec<Person>,
    /// Requests from this account.
    pub outgoing: Vec<Person>,
    /// Accounts this account blocked.
    pub blocked: Vec<Person>,
    /// Groups this account is in.
    pub groups: Vec<GroupInfo>,
    /// Groups this account is invited to.
    pub group_invites: Vec<GroupInvite>,
    /// Table and team conversations this account is in now.
    pub tables: Vec<TableConv>,
    /// Open table invites to this account.
    pub invites: Vec<TableInvite>,
    /// The account's social settings.
    pub settings: SocialSettings,
}

/// A friend.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Friend {
    /// Who.
    pub person: Person,
    /// Since when, Unix milliseconds.
    pub since: u64,
    /// Their presence now.
    pub presence: Presence,
}

/// A group, with its members and pending invites.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupInfo {
    /// The group's id (a UUID); its conversation is `grp:<id>`.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Its members, in the order they joined.
    pub members: Vec<Member>,
    /// Accounts invited and not yet answered.
    pub invited: Vec<Person>,
}

/// A member of a group.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Member {
    /// Who.
    pub person: Person,
    /// Whether they own the group.
    pub owner: bool,
}

/// An invite to a group, as the invitee sees it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupInvite {
    /// The group's id.
    pub group: String,
    /// Its name.
    pub name: String,
    /// Who invited.
    pub inviter: Person,
    /// When, Unix milliseconds.
    pub at: u64,
}

/// A table or team conversation this account is in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableConv {
    /// `tbl:<game>` or `team:<game>:<n>`.
    pub conv: String,
    /// The table's name.
    pub name: String,
    /// Its members now.
    pub members: Vec<Person>,
}

/// An invite to sit at, or watch, a table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableInvite {
    /// The invite's id; `POST /lobby/games/{game}/join` takes it as `invite`.
    pub id: String,
    /// Who invited.
    pub from: Person,
    /// The lobby game.
    pub game: String,
    /// Join a waiting room, or watch a running table.
    pub kind: InviteKind,
    /// The table as it was when invited.
    pub table: TableBrief,
    /// Seconds until it lapses.
    pub expires_in: u32,
}

/// What a table invite is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InviteKind {
    /// Sit down in a waiting room.
    Join,
    /// Watch a running table.
    Watch,
}

/// How a table invite was answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InviteAnswer {
    /// The invitee joined with it.
    Accepted,
    /// The invitee said no.
    Declined,
}

/// A table, briefly.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableBrief {
    /// Its name.
    pub name: String,
    /// Its chairs.
    pub seats: u8,
    /// Its chairs taken.
    pub taken: u8,
    /// `waiting`, `playing` or `over`.
    pub state: String,
    /// Whether it has a password.
    pub locked: bool,
}

/// The settings the gateway enforces for an account.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialSettings {
    /// Who may whisper this account.
    pub whispers_from: WhispersFrom,
    /// Whether others may send this account a friend request.
    pub requests_open: bool,
    /// Whether friends are shown which table this account is at.
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Notice {
    /// Someone joined the group.
    MemberJoined {
        /// Who.
        person: Person,
    },
    /// Someone left the group or was removed.
    MemberLeft {
        /// Who.
        person: Person,
    },
    /// The group was renamed.
    Renamed {
        /// Its new name.
        name: String,
    },
    /// The group has a new owner.
    OwnerNow {
        /// Who.
        person: Person,
    },
}

/// Why something was refused.
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
    /// A recipient device holds [`SLOTS_PER_DEVICE`] undelivered messages.
    RecipientFull,
    /// This gateway has no chat.
    Off,
}

// ---------------------------------------------------------------------------
// HTTP bodies (§8.3), shared so neither end spells them twice.
// ---------------------------------------------------------------------------

/// `POST /social/devices`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceRegistration {
    /// The Ed25519 identity key, 32 bytes, base64url.
    pub ik: String,
    /// The X25519 key, 32 bytes, base64url.
    pub dk: String,
    /// The statement's signature (§4.1), 64 bytes, base64url.
    pub dsig: String,
    /// The OS family (`macOS`), at most [`MAX_DEVICE_LABEL`] characters.
    pub label: String,
}

/// The answer to `POST /social/devices` (`201`), and to anything else that
/// creates something with an id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Created {
    /// The new thing's id.
    pub id: String,
}

/// One of this account's devices, as `GET /social/devices` lists it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceRow {
    /// The device id (its fingerprint).
    pub id: String,
    /// The OS family.
    pub label: String,
    /// When it was registered, Unix milliseconds.
    pub added_at: u64,
    /// When it last opened the socket, Unix milliseconds.
    pub seen_at: u64,
}

/// `GET /social/devices`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceList {
    /// This account's devices, oldest first.
    pub devices: Vec<DeviceRow>,
}

/// One device's public keys, as `GET /social/keys` gives them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceKey {
    /// The device id.
    pub id: String,
    /// The Ed25519 identity key, base64url.
    pub ik: String,
    /// The X25519 key, base64url.
    pub dk: String,
    /// The statement's signature, base64url: check it (§4.1).
    pub dsig: String,
    /// The OS family.
    pub label: String,
    /// When it was registered, Unix milliseconds.
    pub added_at: u64,
}

/// One account's devices.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountKeys {
    /// The tag.
    pub tag: String,
    /// The handle.
    pub handle: String,
    /// Its devices.
    pub devices: Vec<DeviceKey>,
}

/// `GET /social/keys?tag=…`: only the tags this account may write to; any
/// other tag is left out rather than refused.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeysAnswer {
    /// One entry per tag that may be written to.
    pub keys: Vec<AccountKeys>,
}

/// `GET /social/players/{handle}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct PlayerLookup {
    /// The tag.
    pub tag: String,
    /// The handle.
    pub handle: String,
    /// Whether they are this account's friend.
    pub friend: bool,
    /// Whether they asked this account.
    pub incoming: bool,
    /// Whether this account asked them.
    pub outgoing: bool,
    /// Whether this account blocked them.
    pub blocked: bool,
}

/// `POST /social/friends/requests`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriendRequest {
    /// `Name#tag`.
    pub handle: String,
}

/// The answer to a friend request (`201`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestAnswer {
    /// `pending`, or `friends` when they had asked already.
    pub state: RequestState,
}

/// Where a friend request stands.
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagBody {
    /// The tag.
    pub tag: String,
}

/// A body that names a group (`POST /social/groups`, `PATCH …/{id}`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupName {
    /// The name ([`crate::names::group_name`]).
    pub name: String,
}

/// `POST /social/invites`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InviteRequest {
    /// The friend to invite.
    pub tag: String,
    /// The lobby game the caller sits in.
    pub game: String,
}

/// `DELETE /social/invites/{id}`'s optional body.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InviteDecline {
    /// `declined`.
    pub answer: InviteAnswer,
}

/// `POST /social/reports`: one delivered message, revealed (§4.7).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatReport {
    /// The conversation it was delivered in.
    pub conv: String,
    /// The message as it was delivered.
    pub sealed: Delivered,
    /// Its message key, 32 bytes, base64url.
    pub mk: String,
    /// Its decrypted body, base64url of the exact bytes.
    pub body: String,
    /// What the reporter says, at most [`MAX_REPORT_NOTE`] characters.
    #[serde(default)]
    pub note: String,
}

/// The answer to `POST /social/reports` (`201`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportReceipt {
    /// The feedback service's id for it.
    pub report_id: String,
}

/// `GET /info`'s `social`, or `null` when the gateway has no chat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialInfo {
    /// [`SOCIAL_VERSION`].
    pub version: u32,
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
        // 50 members × 5 devices, less the sender's: 249 wraps of 48 bytes.
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
            wraps: vec![wrap; MAX_GROUP_MEMBERS * MAX_DEVICES - 1],
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
