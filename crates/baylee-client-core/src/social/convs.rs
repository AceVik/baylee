//! Conversations and their lines (§10.1): what a tab holds, what a message
//! says once opened, its unread count and whether it is muted.

use serde::{Deserialize, Serialize};

use baylee_protocol::social::{Conv, MAX_TEXT_CHARS, Person};

use super::text;
use super::words::ChatPhrase;

/// A conversation's id: the protocol's [`Conv`] (`dm:…`, `grp:…`, `tbl:…`,
/// `team:…`).
pub type ConvId = Conv;

/// The most lines a conversation holds in memory; older ones stay in the
/// history file (§10.2).
pub const LINES_KEPT: usize = 500;

/// One line, named: its conversation and its id (a message's `UUIDv7`, or
/// `local-<n>` for a line this client wrote itself).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LineKey {
    /// The conversation.
    pub conv: ConvId,
    /// The line's id.
    pub id: String,
}

/// A message's body (§4.2): its text, the cards it links and the players it
/// mentions. Spans are in characters, brackets included, as
/// [`crate::bugreport::refs::CardMention`] counts them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Body {
    /// The text, at most [`MAX_TEXT_CHARS`] characters.
    pub text: String,
    /// Card links.
    #[serde(default)]
    pub links: Vec<BodyLink>,
    /// Mentions of players.
    #[serde(default)]
    pub mentions: Vec<BodyMention>,
    /// `/me`: drawn as `* Name text`.
    #[serde(default)]
    pub emote: bool,
}

/// A card a body links: `[Name]` at `at`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BodyLink {
    /// `[start, end)` in characters, brackets included.
    pub at: [usize; 2],
    /// The append-only ledger's `CardIndex`.
    pub card: u32,
    /// The face showing.
    #[serde(default)]
    pub face: u8,
    /// The printing, when the sender had one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print: Option<BodyPrint>,
}

/// A printing, as a link names it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BodyPrint {
    /// Scryfall's printing id.
    pub scryfall_id: String,
    /// The printing's language.
    pub lang: String,
}

/// A player a body mentions: `[@Name#tag]` at `at`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BodyMention {
    /// `[start, end)` in characters, brackets included.
    pub at: [usize; 2],
    /// The player's tag.
    pub tag: String,
}

impl Body {
    /// A received body as it may be drawn (§4.2 step 5): the text cleaned
    /// ([`text::clean`]), and spans kept only when they lie inside the text,
    /// in order, without overlapping, and the text needed no cleaning (a
    /// cleaned text's characters moved under them). `None` for a body with
    /// no text at all.
    #[must_use]
    pub fn checked(self) -> Option<Self> {
        let moved = text::needs_cleaning(&self.text);
        let text = text::clean(&self.text);
        if text.trim().is_empty() {
            return None;
        }
        let len = text.chars().count();
        let fits = |at: [usize; 2]| at[0] < at[1] && at[1] <= len;
        let (links, mentions) = if moved {
            (Vec::new(), Vec::new())
        } else {
            let mut links: Vec<BodyLink> = self.links.into_iter().filter(|l| fits(l.at)).collect();
            let mut mentions: Vec<BodyMention> = self
                .mentions
                .into_iter()
                .filter(|m| fits(m.at) && baylee_protocol::social::valid_tag(&m.tag))
                .collect();
            links.sort_by_key(|l| l.at);
            mentions.sort_by_key(|m| m.at);
            let mut spans: Vec<[usize; 2]> = links
                .iter()
                .map(|l| l.at)
                .chain(mentions.iter().map(|m| m.at))
                .collect();
            spans.sort_unstable();
            let overlaps = spans.windows(2).any(|w| w[0][1] > w[1][0]);
            if overlaps {
                (Vec::new(), Vec::new())
            } else {
                (links, mentions)
            }
        };
        Some(Self {
            text,
            links,
            mentions,
            emote: self.emote,
        })
    }

    /// Whether `tag` is mentioned.
    #[must_use]
    pub fn mentions_tag(&self, tag: &str) -> bool {
        self.mentions.iter().any(|m| m.tag == tag)
    }

    /// The characters of `at` in the text.
    #[must_use]
    pub fn span_text(&self, at: [usize; 2]) -> String {
        self.text
            .chars()
            .skip(at[0])
            .take(at[1].saturating_sub(at[0]))
            .collect()
    }
}

/// Where a line is in its life.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineState {
    /// Delivered, or sent and stored by the gateway.
    Final,
    /// Sent and not yet answered: drawn at 60 % with `◌`.
    Pending,
    /// Not sent: drawn with `!`; its menu offers Retry and Remove.
    Failed,
}

/// What a line is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LineKind {
    /// A message, received or sent.
    Message {
        /// Who wrote it (handle from the gateway, never from the body).
        from: Person,
        /// Whether this account wrote it.
        mine: bool,
        /// What it says.
        body: Body,
    },
    /// A line this client writes (a device change, a refusal, a command's
    /// answer): never a message, never sent.
    Notice {
        /// What it says.
        phrase: ChatPhrase,
        /// Its placeholders' values.
        args: Vec<String>,
    },
    /// A line the gateway wrote (a member joined, a rename): drawn as a
    /// server notice.
    Server {
        /// What it says.
        phrase: ChatPhrase,
        /// Its placeholders' values.
        args: Vec<String>,
    },
    /// "New table": a rematch room began.
    Divider,
}

/// One line of a conversation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatLine {
    /// Its name.
    pub key: LineKey,
    /// When: the gateway's receipt time for a message, the local clock for
    /// a line written here, Unix milliseconds.
    pub at: u64,
    /// What it is.
    pub kind: LineKind,
    /// Sent, pending or failed.
    pub state: LineState,
    /// From an ignored player: drawn as "1 hidden message" until opened.
    pub hidden: bool,
    /// Mentions this account: drawn lit.
    pub mentions_me: bool,
    /// Why a failed line failed.
    pub failure: Option<ChatPhrase>,
}

impl ChatLine {
    /// The message's sender's tag, for a message.
    #[must_use]
    pub fn from_tag(&self) -> Option<&str> {
        match &self.kind {
            LineKind::Message { from, .. } => Some(&from.tag),
            _ => None,
        }
    }

    /// The message's body, for a message.
    #[must_use]
    pub fn body(&self) -> Option<&Body> {
        match &self.kind {
            LineKind::Message { body, .. } => Some(body),
            _ => None,
        }
    }
}

/// Which kind of conversation, for notifications and the tab's ink.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConvKind {
    /// Global lobby chat for all online players.
    Global,
    /// A whisper.
    Whisper,
    /// A group.
    Group,
    /// A table.
    Table,
    /// A team at a table.
    Team,
}

impl ConvKind {
    /// The kind of `id`.
    #[must_use]
    pub fn of(id: &ConvId) -> Self {
        match id {
            Conv::Global => Self::Global,
            Conv::Dm { .. } => Self::Whisper,
            Conv::Group { .. } => Self::Group,
            Conv::Table { .. } => Self::Table,
            Conv::Team { .. } => Self::Team,
        }
    }
}

/// One tab: a conversation's lines, unread count and settings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conversation {
    /// Its id.
    pub id: ConvId,
    /// Its title: the partner's handle (or nickname), the group's name,
    /// "Table" or "Team".
    pub title: String,
    /// Its lines, oldest first, at most [`LINES_KEPT`].
    pub lines: Vec<ChatLine>,
    /// Lines that arrived since it was last read.
    pub unread: u32,
    /// Muted: no badge, sound or toast; a dim count only.
    pub muted: bool,
    /// The members, for a table, team or group (the gateway's list).
    pub members: Vec<Person>,
    /// Still open as a tab (a closed whisper reopens on its next line).
    pub open: bool,
    /// When a table or team conversation stopped being this account's
    /// (its table was swept or it left the chair); lines from its members
    /// are accepted for one hour after (§4.2 step 6).
    pub ended_at: Option<u64>,
}

impl Conversation {
    /// A new, empty conversation.
    #[must_use]
    pub fn new(id: ConvId, title: String) -> Self {
        Self {
            id,
            title,
            lines: Vec::new(),
            unread: 0,
            muted: false,
            members: Vec::new(),
            open: true,
            ended_at: None,
        }
    }

    /// Its kind.
    #[must_use]
    pub fn kind(&self) -> ConvKind {
        ConvKind::of(&self.id)
    }

    /// Puts `line` in order (by time, then id: the gateway's order, §4.6)
    /// and keeps at most [`LINES_KEPT`]. A line with a key already here
    /// replaces it.
    pub fn insert(&mut self, line: ChatLine) {
        // A pending line that became final may move: the gateway's time
        // replaces the local one.
        self.lines.retain(|l| l.key != line.key);
        let at = self
            .lines
            .partition_point(|l| (l.at, &l.key.id) <= (line.at, &line.key.id));
        self.lines.insert(at, line);
        if self.lines.len() > LINES_KEPT {
            let extra = self.lines.len() - LINES_KEPT;
            self.lines.drain(..extra);
        }
    }

    /// The line named `id`.
    #[must_use]
    pub fn line(&self, id: &str) -> Option<&ChatLine> {
        self.lines.iter().find(|l| l.key.id == id)
    }

    /// The line named `id`, to change.
    pub fn line_mut(&mut self, id: &str) -> Option<&mut ChatLine> {
        self.lines.iter_mut().find(|l| l.key.id == id)
    }

    /// Whether `tag` is a member now.
    #[must_use]
    pub fn has_member(&self, tag: &str) -> bool {
        self.members.iter().any(|m| m.tag == tag)
    }
}

/// The longest text a composer sends, in characters (§8.5).
pub const COMPOSE_LIMIT: usize = MAX_TEXT_CHARS;
