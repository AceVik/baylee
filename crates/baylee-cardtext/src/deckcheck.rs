//! What a deck check says, and what an own ban list holds, in the words both
//! ends read.
//!
//! `baylee_cards::formats::validate` is the one function that decides; this
//! is its vocabulary, here for the reason [`crate::PoolFactsBody`] is: the
//! gateway answers these shapes, a client draws them, and this crate depends
//! on serde and nothing else. Legality is the gateway's question and never a
//! rule of the game, so none of it is in the engine
//! (`docs/banlists-and-formats.md`).
//!
//! A card is its registry index here, as in [`crate::PoolFactsBody`], and a
//! list keys on the oracle id, because a list outlives a build and names
//! cards this pool may not hold yet.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The longest name a list may have, in characters.
pub const MAX_LIST_NAME_CHARS: usize = 60;
/// The most cards an own list may hold. An official Vintage list is about
/// sixty; this leaves a host room and bounds what a room has to hold.
pub const MAX_LIST_ENTRIES: usize = 2000;
/// The most lists one account may own.
pub const MAX_LISTS_PER_ACCOUNT: usize = 50;

/// What a list says about a card.
///
/// @description What a list says about a card: `banned` out of every deck,
/// `restricted` one copy, `banned_as_commander` in the deck but not leading
/// it.
/// @openapi
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListStatus {
    /// Out of every deck.
    Banned,
    /// One copy, as in Vintage.
    Restricted,
    /// In the deck, but not leading it.
    BannedAsCommander,
}

impl ListStatus {
    /// The word the store and the wire write.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Banned => "banned",
            Self::Restricted => "restricted",
            Self::BannedAsCommander => "banned_as_commander",
        }
    }

    /// The status a word names.
    #[must_use]
    pub fn of(word: &str) -> Option<Self> {
        [Self::Banned, Self::Restricted, Self::BannedAsCommander]
            .into_iter()
            .find(|status| status.word() == word)
    }
}

/// A list as a check reads it: what it says about each card, by oracle id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListRules {
    statuses: BTreeMap<String, ListStatus>,
}

impl ListRules {
    /// What the list says about the card with this oracle id.
    #[must_use]
    pub fn status(&self, oracle_id: &str) -> Option<ListStatus> {
        self.statuses.get(oracle_id).copied()
    }

    /// How many cards it names.
    #[must_use]
    pub fn len(&self) -> usize {
        self.statuses.len()
    }

    /// Whether it names none, which is a list that bans nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.statuses.is_empty()
    }
}

impl FromIterator<(String, ListStatus)> for ListRules {
    fn from_iter<I: IntoIterator<Item = (String, ListStatus)>>(entries: I) -> Self {
        Self {
            statuses: entries.into_iter().collect(),
        }
    }
}

/// Why a deck is not what it was checked against.
///
/// @description One reason a deck fails a check; `kind` says which, and the
/// other fields are that kind's. A card is its registry index, as in
/// `GET /pool`. A client words each kind itself.
/// @openapi
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Problem {
    /// A card the pool has no definition for.
    UnknownCard {
        /// The card.
        card: u32,
    },
    /// Fewer cards than the shape needs.
    TooFew {
        /// Cards in the deck, leaders included.
        have: u32,
        /// The least the shape takes.
        need: u32,
    },
    /// Not the exact size a singleton shape takes.
    WrongSize {
        /// Cards in the deck, leaders included.
        have: u32,
        /// The size the shape takes.
        need: u32,
    },
    /// More copies of a card than the shape allows.
    TooManyCopies {
        /// The card.
        card: u32,
        /// Copies in the deck and its sideboard.
        have: u32,
        /// Copies allowed.
        limit: u32,
    },
    /// A sideboard over what the shape takes; a Commander deck takes none.
    SideboardTooBig {
        /// Cards in the sideboard.
        have: u32,
        /// The most the shape takes.
        limit: u32,
    },
    /// A Commander deck without a leader, or with more than two.
    Leaders {
        /// Leaders named.
        have: u32,
    },
    /// A leader that may not lead a deck.
    NotALeader {
        /// The card.
        card: u32,
    },
    /// Two leaders that may not lead together.
    CannotLeadTogether {
        /// One of them.
        card: u32,
        /// The other.
        with: u32,
    },
    /// A card outside the leaders' colour identity.
    OutsideIdentity {
        /// The card.
        card: u32,
    },
    /// Leaders were named for a shape that has none.
    LeadersNotTaken,
    /// The format bans the card.
    Banned {
        /// The card.
        card: u32,
    },
    /// The format restricts the card to one copy, and the deck holds more.
    Restricted {
        /// The card.
        card: u32,
        /// Copies in the deck and its sideboard.
        have: u32,
    },
    /// The format does not have the card at all.
    NotLegal {
        /// The card.
        card: u32,
    },
    /// The chosen list bans the card.
    ListBanned {
        /// The card.
        card: u32,
    },
    /// The chosen list restricts the card to one copy, and the deck holds
    /// more.
    ListRestricted {
        /// The card.
        card: u32,
        /// Copies in the deck and its sideboard.
        have: u32,
    },
    /// The chosen list bans the card as a commander, and it leads the deck.
    ListBannedAsCommander {
        /// The card.
        card: u32,
    },
}

impl Problem {
    /// The card this problem is about, where it is about one: what a list of
    /// problems points at.
    #[must_use]
    pub const fn card(self) -> Option<u32> {
        match self {
            Self::UnknownCard { card }
            | Self::TooManyCopies { card, .. }
            | Self::NotALeader { card }
            | Self::CannotLeadTogether { card, .. }
            | Self::OutsideIdentity { card }
            | Self::Banned { card }
            | Self::Restricted { card, .. }
            | Self::NotLegal { card }
            | Self::ListBanned { card }
            | Self::ListRestricted { card, .. }
            | Self::ListBannedAsCommander { card } => Some(card),
            Self::TooFew { .. }
            | Self::WrongSize { .. }
            | Self::SideboardTooBig { .. }
            | Self::Leaders { .. }
            | Self::LeadersNotTaken => None,
        }
    }

    /// Whether this problem names `card`, the second of a pair included.
    #[must_use]
    pub fn concerns(self, card: u32) -> bool {
        self.card() == Some(card)
            || matches!(self, Self::CannotLeadTogether { with, .. } if with == card)
    }
}

/// What could not be asked, and so stands between a deck and the word legal.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Unchecked {
    /// No format verdicts to read: offline, a gateway with no catalog, or a
    /// format the table does not hold.
    pub statuses: bool,
    /// The format's deck shape is not one this build knows, so size, copies
    /// and leaders were not asked.
    pub shape: bool,
    /// Cards the table says nothing about in this format, by registry index.
    pub cards: Vec<u32>,
}

impl Unchecked {
    /// Whether something was left unasked.
    #[must_use]
    pub const fn any(&self) -> bool {
        self.statuses || self.shape || !self.cards.is_empty()
    }
}

/// What a check comes to, in one word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    /// Everything was asked and nothing is wrong.
    Legal,
    /// Something is wrong.
    Illegal,
    /// Nothing wrong was found, but not everything could be asked: a deck
    /// that is never called legal.
    Unknown,
}

/// What one check found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Everything that makes the deck illegal, shape first, then by card.
    pub problems: Vec<Problem>,
    /// Cards with no script, once each: a notice and never a legality fault.
    /// A format-legal deck may hold one.
    pub unscripted: Vec<u32>,
    /// What could not be asked.
    pub unchecked: Unchecked,
}

impl Report {
    /// The deck's standing.
    #[must_use]
    pub fn standing(&self) -> Standing {
        if !self.problems.is_empty() {
            Standing::Illegal
        } else if self.unchecked.any() {
            Standing::Unknown
        } else {
            Standing::Legal
        }
    }

    /// The problems that name this card.
    pub fn of(&self, card: u32) -> impl Iterator<Item = Problem> + '_ {
        self.problems
            .iter()
            .copied()
            .filter(move |problem| problem.concerns(card))
    }
}

/// Why a table's list turned a deck away, in a refusal's `refused`.
///
/// @description Why a table's list turned a deck away: the list, the format
/// and every problem the deck has against them. A client words them; `error`
/// says the same in English for one that cannot.
/// @openapi
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeckRefusal {
    /// @description The list's name; an official list's is its format's key.
    pub list: String,
    /// @description The format the list keeps to.
    pub format: String,
    /// @description What is wrong with the deck.
    pub problems: Vec<Problem>,
}

/// A list, as `GET /banlists` lists it.
///
/// @description A ban list: an official one (`builtin`), which follows the
/// card feed and nobody edits, or the caller's own. Nothing says whose a
/// list is beyond `mine`.
/// @openapi
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListRow {
    /// @description The list's id.
    pub id: String,
    /// @description Its name; an official list's is its format's key.
    pub name: String,
    /// @description The format it is for: Scryfall's key, or `freeform` for
    /// a list that keeps to no format.
    pub format: String,
    /// @description Whether it is an official list.
    pub builtin: bool,
    /// @description Whether it is the caller's own.
    pub mine: bool,
    /// @description Counts up with every change: what a game freezes.
    pub version: i64,
    /// @description When it last changed, in unix seconds.
    pub updated_at: u64,
    /// @description How many cards it names.
    pub cards: u32,
    /// @description The registry index of each of them this pool holds, on
    /// `GET /banlists` alone: what `list:` searches by. `GET /banlists/{id}`
    /// says it per entry instead.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub indices: Vec<u32>,
}

/// One card on a list.
///
/// @description One card on a list.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListEntry {
    /// @description The card's oracle id, which is what a list is keyed on.
    pub oracle_id: String,
    /// @description What the list says about it.
    pub status: ListStatus,
    /// @description Its registry index, `null` for a card this build has no
    /// definition for.
    #[serde(default)]
    pub index: Option<u32>,
    /// @description Its English name, empty where this gateway knows none.
    #[serde(default)]
    pub name: String,
}

/// `GET /banlists/{id}`: a list and every card on it.
///
/// @description A list and every card on it.
/// @openapi
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListBody {
    /// @description The list.
    #[serde(flatten)]
    pub row: ListRow,
    /// @description Its cards, by oracle id.
    pub entries: Vec<ListEntry>,
}

impl ListBody {
    /// The list as a check reads it.
    #[must_use]
    pub fn rules(&self) -> ListRules {
        self.entries
            .iter()
            .map(|entry| (entry.oracle_id.clone(), entry.status))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_status_is_read_by_the_word_it_is_written_as() {
        for status in [
            ListStatus::Banned,
            ListStatus::Restricted,
            ListStatus::BannedAsCommander,
        ] {
            assert_eq!(ListStatus::of(status.word()), Some(status));
            let written = serde_json::to_string(&status).expect("writes");
            assert_eq!(written, format!("\"{}\"", status.word()));
        }
        assert_eq!(ListStatus::of("legal"), None);
    }

    /// The refusal a client reads is the shape the gateway writes: pinned, as
    /// [`crate::PoolFactsBody`] is.
    #[test]
    fn a_problem_says_its_kind_and_is_read_back() {
        let problem = Problem::ListRestricted { card: 7, have: 3 };
        let written = serde_json::to_string(&problem).expect("writes");
        assert_eq!(written, r#"{"kind":"list_restricted","card":7,"have":3}"#);
        let read: Problem = serde_json::from_str(&written).expect("reads");
        assert_eq!(read, problem);
    }

    #[test]
    fn a_standing_needs_everything_asked() {
        let mut report = Report::default();
        assert_eq!(report.standing(), Standing::Legal);
        report.unchecked.statuses = true;
        assert_eq!(report.standing(), Standing::Unknown, "never called legal");
        report.problems.push(Problem::LeadersNotTaken);
        assert_eq!(report.standing(), Standing::Illegal);
    }

    #[test]
    fn a_pair_of_leaders_concerns_both() {
        let report = Report {
            problems: vec![Problem::CannotLeadTogether { card: 1, with: 2 }],
            ..Report::default()
        };
        assert_eq!(report.of(2).count(), 1);
        assert_eq!(report.of(1).count(), 1);
        assert_eq!(report.of(3).count(), 0);
    }

    #[test]
    fn a_list_body_reads_as_rules() {
        let body = ListBody {
            entries: vec![ListEntry {
                oracle_id: "a".to_string(),
                status: ListStatus::Restricted,
                index: None,
                name: String::new(),
            }],
            ..ListBody::default()
        };
        assert_eq!(body.rules().status("a"), Some(ListStatus::Restricted));
        assert_eq!(body.rules().status("b"), None);
    }
}
