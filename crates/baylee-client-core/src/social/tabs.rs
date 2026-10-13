//! The chat window's fixed tabs and what each shows (the chat window
//! design, §3): All · Log · Table · Team · Private · Group, each with a
//! filter of the kinds of line it admits.
//!
//! The tabs are fixed in v1: a player picks only what a tab shows
//! ([`TabFilter`]), never its name or where its composer writes. The
//! filters ride in [`super::ChatPrefs::tabs`], which travels with the
//! account (`Preferences.chat`).

use serde::{Deserialize, Serialize};

use baylee_protocol::social::Conv;

use super::convs::{ChatLine, ConvId, LineKind};

/// One of the chat window's tabs, in drawn order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatTab {
    /// Everything the other tabs show, the game log included.
    All,
    /// The game log.
    Log,
    /// The table's chat.
    Table,
    /// The team's chat, at a team table.
    Team,
    /// Every whisper, interleaved.
    Private,
    /// The shown group's lines.
    Group,
}

impl ChatTab {
    /// Every tab, in the order they are drawn.
    pub const ALL: [Self; 6] = [
        Self::All,
        Self::Log,
        Self::Table,
        Self::Team,
        Self::Private,
        Self::Group,
    ];

    /// Its place in [`Self::ALL`] (and in [`super::ChatPrefs::tabs`]).
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Self::All => 0,
            Self::Log => 1,
            Self::Table => 2,
            Self::Team => 3,
            Self::Private => 4,
            Self::Group => 5,
        }
    }

    /// A tab the player writes in (not All, not Log).
    #[must_use]
    pub fn is_chat(self) -> bool {
        !matches!(self, Self::All | Self::Log)
    }

    /// Its wire spelling, for `/state.chat`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Log => "log",
            Self::Table => "table",
            Self::Team => "team",
            Self::Private => "private",
            Self::Group => "group",
        }
    }

    /// The tab a conversation's lines are read on.
    #[must_use]
    pub fn of(conv: &ConvId) -> Self {
        match conv {
            Conv::Dm { .. } => Self::Private,
            Conv::Group { .. } => Self::Group,
            Conv::Table { .. } => Self::Table,
            Conv::Team { .. } => Self::Team,
        }
    }
}

/// Which groups a tab shows.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupPick {
    /// Every group.
    #[default]
    All,
    /// Only these groups, by id.
    Only(Vec<String>),
    /// None.
    None,
}

impl GroupPick {
    /// Whether the group `id` is shown.
    #[must_use]
    pub fn admits(&self, id: &str) -> bool {
        match self {
            Self::All => true,
            Self::Only(ids) => ids.iter().any(|i| i == id),
            Self::None => false,
        }
    }
}

/// What one tab shows (§3.5): a set of kinds of line, and whether its
/// lines carry their time.
#[allow(clippy::struct_excessive_bools)] // one tick box each, as the dialog draws them
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TabFilter {
    /// The game log's lines. Stored as `game_log`: a bare `log` key in the
    /// preferences a report carries read as the report's own log part.
    #[serde(rename = "game_log", alias = "log")]
    pub log: bool,
    /// Table chat.
    pub table: bool,
    /// Team chat.
    pub team: bool,
    /// Whispers.
    pub whispers: bool,
    /// Group lines.
    pub groups: GroupPick,
    /// Notices: joins and leaves, device changes, invites, "New table".
    pub notices: bool,
    /// A friend coming online or going offline (the lobby; off by default).
    pub friends_presence: bool,
    /// Each line's time.
    pub timestamps: bool,
}

impl Default for TabFilter {
    fn default() -> Self {
        Self::nothing()
    }
}

impl TabFilter {
    /// A filter that admits nothing, with timestamps.
    #[must_use]
    pub const fn nothing() -> Self {
        Self {
            log: false,
            table: false,
            team: false,
            whispers: false,
            groups: GroupPick::None,
            notices: false,
            friends_presence: false,
            timestamps: true,
        }
    }

    /// The filter `tab` starts with (§3.1's "Shows" column).
    #[must_use]
    pub fn standard(tab: ChatTab) -> Self {
        let base = Self::nothing();
        match tab {
            ChatTab::All => Self {
                log: true,
                table: true,
                team: true,
                whispers: true,
                groups: GroupPick::All,
                notices: true,
                ..base
            },
            ChatTab::Log => Self { log: true, ..base },
            ChatTab::Table => Self {
                table: true,
                notices: true,
                ..base
            },
            ChatTab::Team => Self {
                team: true,
                notices: true,
                ..base
            },
            ChatTab::Private => Self {
                whispers: true,
                notices: true,
                ..base
            },
            ChatTab::Group => Self {
                groups: GroupPick::All,
                notices: true,
                ..base
            },
        }
    }

    /// Every tab's starting filter, in [`ChatTab::ALL`]'s order.
    #[must_use]
    pub fn standard_set() -> Vec<Self> {
        ChatTab::ALL.iter().map(|t| Self::standard(*t)).collect()
    }

    /// Whether a conversation's lines may show at all under this filter.
    #[must_use]
    pub fn admits_conv(&self, conv: &ConvId) -> bool {
        match conv {
            Conv::Dm { .. } => self.whispers,
            Conv::Group { id } => self.groups.admits(id),
            Conv::Table { .. } => self.table,
            Conv::Team { .. } => self.team,
        }
    }

    /// Whether `line` of `conv` shows. A message or a line this client wrote
    /// (a command's answer, a refusal) shows where its conversation does;
    /// a server notice and the "New table" divider need
    /// [`Self::notices`] too.
    #[must_use]
    pub fn admits(&self, conv: &ConvId, line: &ChatLine) -> bool {
        if !self.admits_conv(conv) {
            return false;
        }
        match line.kind {
            LineKind::Message { .. } | LineKind::Notice { .. } => true,
            LineKind::Server { .. } | LineKind::Divider => self.notices,
        }
    }

    /// Whether this filter admits nothing at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.log
            && !self.table
            && !self.team
            && !self.whispers
            && self.groups == GroupPick::None
            && !self.notices
            && !self.friends_presence
    }
}
