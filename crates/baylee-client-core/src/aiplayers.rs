//! AI players in Ranked (`docs/proposals/ai-players.md`), as a person meets
//! them: whom a search accepts, the fill-in countdown, and the badge every
//! AI wears wherever its name is drawn.
//!
//! Everything here is policy with no renderer, so it is tested as arithmetic:
//! which words a countdown says at a given second, which mark a kind wears,
//! what a badge is called aloud, and how a remembered choice is read back.
//!
//! **Nothing is preselected.** A person who has never chosen has
//! [`Opponents`] `None`, and only a stored, explicit choice ever lets a
//! search accept an AI ([`Opponents::wire`]); `None` on the wire is people
//! only, which is what Central reads from a client that says nothing.
//!
//! **Only Central names an AI.** A badge's label is an administrator's text
//! relayed by Central, so it is cleaned before it is drawn ([`label`]), as a
//! received name is.

use crate::decisionclock::mmss;
use crate::i18n::{Lang, Phrase};
pub use central_api::ranked::ai::{AiBadge, AiKind, AiOpponents, BoardPlayers, LABEL_CHARS};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Whom a person's ranked search accepts, as they chose it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Opponents {
    /// People only ("Nur Menschen"): no AI fills the search.
    People,
    /// The house levels and Baylee AI: the moves stay on the operator's
    /// servers.
    Local,
    /// Also language models, whose providers receive the game's moves.
    Any,
}

impl Opponents {
    /// The three choices, in the order they are offered.
    pub const ALL: [Self; 3] = [Self::People, Self::Local, Self::Any];

    /// What the search sends Central.
    #[must_use]
    pub const fn wire(self) -> AiOpponents {
        match self {
            Self::People => AiOpponents::None,
            Self::Local => AiOpponents::Local,
            Self::Any => AiOpponents::Any,
        }
    }

    /// Whether an AI may fill the search.
    #[must_use]
    pub const fn accepts_ai(self) -> bool {
        !matches!(self, Self::People)
    }

    /// The choice's short name, as a segment and a sheet's option say it.
    #[must_use]
    pub const fn title(self) -> Phrase {
        match self {
            Self::People => Phrase::RankedOpponentsPeople,
            Self::Local => Phrase::RankedOpponentsLocal,
            Self::Any => Phrase::RankedOpponentsAny,
        }
    }

    /// The one plain line that says what the choice means; for
    /// [`Self::Any`], that a language model's provider receives the moves.
    #[must_use]
    pub const fn meaning(self) -> Phrase {
        match self {
            Self::People => Phrase::RankedOpponentsPeopleHelp,
            Self::Local => Phrase::RankedOpponentsLocalHelp,
            Self::Any => Phrase::RankedOpponentsAnyHelp,
        }
    }

    /// Its position in [`Self::ALL`].
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::People => 0,
            Self::Local => 1,
            Self::Any => 2,
        }
    }
}

/// The choice each Central account made on this device, by its public id.
///
/// By account and not for the device: consent belongs to the person who
/// gave it, and a second person signing in here has not given it. A file
/// from before it, or one naming a choice this build does not know, reads
/// as never asked ([`Self::of`]), which asks again rather than guessing.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Remembered(BTreeMap<String, String>);

impl Remembered {
    /// The most accounts one device remembers; the oldest key in sort
    /// order gives way, which bounds a file nobody prunes.
    pub const MAX: usize = 32;

    /// Whether no account chose here yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// What `account` chose here, or `None` when it never chose.
    #[must_use]
    pub fn of(&self, account: &str) -> Option<Opponents> {
        let stored = self.0.get(account)?;
        serde_json::from_value(serde_json::Value::String(stored.clone())).ok()
    }

    /// Remembers `choice` for `account`; `true` when that changed anything.
    pub fn keep(&mut self, account: &str, choice: Opponents) -> bool {
        let Ok(serde_json::Value::String(word)) = serde_json::to_value(choice) else {
            return false;
        };
        if self.0.get(account) == Some(&word) {
            return false;
        }
        self.0.insert(account.to_owned(), word);
        while self.0.len() > Self::MAX {
            let Some(first) = self.0.keys().find(|key| key.as_str() != account).cloned() else {
                break;
            };
            self.0.remove(&first);
        }
        true
    }
}

/// How long before Central's fill-in time the countdown shows, in seconds:
/// the concept's "after 45 s a 15 s countdown" with Central's default of 60.
pub const COUNTDOWN_SECS: i64 = 15;

/// Where a waiting search stands against Central's fill-in time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillIn {
    /// No AI will be seated: the search takes none, or no AI is offered.
    Off,
    /// An AI may come, but the countdown has not begun.
    Later,
    /// The countdown, in whole seconds left (1 to [`COUNTDOWN_SECS`]).
    Counting(u32),
    /// The time has come: the search goes on as before until Central seats
    /// an AI or a person. Never "in 0:00".
    Due,
}

impl FillIn {
    /// Where the search stands at `now` (Central's clock, Unix seconds)
    /// against `at`, the `Waiting` reply's `ai_fill_in_at`.
    #[must_use]
    pub fn at(now: i64, at: Option<i64>) -> Self {
        let Some(at) = at else {
            return Self::Off;
        };
        let left = at.saturating_sub(now);
        if left <= 0 {
            Self::Due
        } else if left <= COUNTDOWN_SECS {
            Self::Counting(u32::try_from(left).unwrap_or(0))
        } else {
            Self::Later
        }
    }

    /// The countdown's words, "AI opponent in 0:12"; none outside it.
    #[must_use]
    pub fn words(self, lang: Lang) -> Option<String> {
        match self {
            Self::Counting(secs) => Some(Phrase::RankedFillIn.fill(lang, &[&mmss(secs)])),
            Self::Off | Self::Later | Self::Due => None,
        }
    }

    /// How opaque the countdown is at `now_ms` (Central's clock, Unix
    /// milliseconds) against `at` (Unix seconds): it fades in over
    /// [`FADE_MS`] from the countdown's first second and out over the same
    /// before the time comes, so it is gone when it would say "0:00". From
    /// the clock and not from a component, so a redrawn card carries on
    /// where the last one stood.
    #[must_use]
    pub fn opacity(now_ms: i64, at: Option<i64>) -> f32 {
        let Some(at) = at else {
            return 0.0;
        };
        let start = at.saturating_sub(COUNTDOWN_SECS).saturating_mul(1000);
        let end = at.saturating_mul(1000);
        let rise = now_ms.saturating_sub(start) as f32 / FADE_MS as f32;
        let fall = end.saturating_sub(now_ms) as f32 / FADE_MS as f32;
        rise.min(fall).clamp(0.0, 1.0)
    }
}

/// How long the countdown takes to fade in or out, in milliseconds.
pub const FADE_MS: i64 = 600;

/// Which mark an AI's kind wears. The client draws each from the icon face
/// it ships; a kind a later Central adds is [`Self::Other`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mark {
    /// A house level: the robot, as a house chair wears it at the table.
    House,
    /// A language model: a speech bubble.
    Model,
    /// The trained net: linked nodes.
    Net,
    /// A kind this build does not know: a chip.
    Other,
}

impl Mark {
    /// The mark `kind` wears.
    #[must_use]
    pub const fn of(kind: AiKind) -> Self {
        match kind {
            AiKind::House => Self::House,
            AiKind::Model => Self::Model,
            AiKind::Net => Self::Net,
            AiKind::Other => Self::Other,
        }
    }
}

/// What kind of mind `kind` is, in words.
#[must_use]
pub const fn kind_name(kind: AiKind) -> Phrase {
    match kind {
        AiKind::House => Phrase::AiKindHouse,
        AiKind::Model => Phrase::AiKindModel,
        AiKind::Net => Phrase::AiKindNet,
        AiKind::Other => Phrase::AiKindOther,
    }
}

/// A badge's label as it may be drawn: hidden characters dropped, at most
/// Central's [`LABEL_CHARS`], surrounding space trimmed; an empty one is the
/// kind's own name, so an AI is never drawn nameless.
#[must_use]
pub fn label(badge: &AiBadge, lang: Lang) -> String {
    let clean: String = badge
        .label
        .chars()
        .filter(|c| !crate::social::text::is_hidden(*c) && *c != '\n')
        .take(LABEL_CHARS)
        .collect();
    let clean = clean.trim();
    if clean.is_empty() {
        crate::i18n::sentence_case(kind_name(badge.kind).text(lang)).into_owned()
    } else {
        clean.to_owned()
    }
}

/// The badge read aloud and in its hint: "AI player: Haus-KI · Expert
/// (house AI)".
#[must_use]
pub fn spoken(badge: &AiBadge, lang: Lang) -> String {
    Phrase::AiBadgeSpoken.fill(
        lang,
        &[&label(badge, lang), kind_name(badge.kind).text(lang)],
    )
}

/// The badge's hover hint: who it is, that it plays from its own view, and
/// for a language model where the moves go.
#[must_use]
pub fn hint(badge: &AiBadge, lang: Lang) -> String {
    let mut words = format!(
        "{}\n{}",
        spoken(badge, lang),
        Phrase::AiPlaysFair.text(lang)
    );
    if badge.kind == AiKind::Model {
        words.push('\n');
        words.push_str(Phrase::AiModelProvider.text(lang));
    }
    words
}

/// The ladder's three groups, in the order its control offers them.
pub const BOARD_PLAYERS: [BoardPlayers; 3] = [
    BoardPlayers::Everyone,
    BoardPlayers::People,
    BoardPlayers::Ai,
];

/// A ladder group's name on its segment.
#[must_use]
pub const fn board_players_name(players: BoardPlayers) -> Phrase {
    match players {
        BoardPlayers::Everyone => Phrase::RankedBoardEveryone,
        BoardPlayers::People => Phrase::RankedBoardPeople,
        BoardPlayers::Ai => Phrase::RankedBoardAi,
    }
}

/// A ladder group's position in [`BOARD_PLAYERS`].
#[must_use]
pub const fn board_players_index(players: BoardPlayers) -> usize {
    match players {
        BoardPlayers::Everyone => 0,
        BoardPlayers::People => 1,
        BoardPlayers::Ai => 2,
    }
}

mod record;
pub use record::{Counted, Record, Records, Tally, Verdict};

#[cfg(test)]
mod tests;
