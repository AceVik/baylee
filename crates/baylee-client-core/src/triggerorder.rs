//! The order a player's own simultaneous triggers go on the stack
//! (CR 603.3b): a suggestion the player only has to confirm, why it is so,
//! and what the player taught it.
//!
//! The player decides. The suggestion is made from what is public, by three
//! kinds of knowledge in order of weight, and a heavier one is never undone
//! by a lighter one:
//!
//! 1. the game's end ([`outcome`]): an order that wins, or that does not lose;
//! 2. what the player taught ([`memory`]): pairs of abilities, newest first;
//! 3. rules of good play ([`rules`]), read off the compiled abilities
//!    ([`facts`]) and never off anybody's card text.
//!
//! All of them speak in pairs, "this before that", and [`plan`] keeps the
//! pairs that still fit: a pair that would close a circle is dropped, so
//! contradictions cannot hang and nothing here has to be cleaned up by hand.
//!
//! Everything speaks in **resolution order**: the first of a list resolves
//! first. Which end of the wire that is, is the client's to translate.

pub mod facts;
pub mod memory;
mod outcome;
mod plan;
mod rules;
#[cfg(test)]
mod tests;

use crate::i18n::Phrase;
use baylee_core::ids::{AbilityRef, ObjectId, PlayerId};
use baylee_view::TokenAbility;
use facts::Facts;
use memory::TriggerMemory;
use plan::{Added, Plan};

/// One ability, the same in every game: what a player's teaching is filed
/// under, as their standing answers are. Never an object: an object lives
/// in one game (`docs/card-identity.md`).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum TriggerKey {
    /// An ability a card prints.
    Card(AbilityRef),
    /// An ability of a token from the registry.
    Token {
        /// The token's registry id.
        token: u16,
        /// Index in that token's ability list.
        index: u32,
    },
}

impl From<TokenAbility> for TriggerKey {
    fn from(ability: TokenAbility) -> Self {
        Self::Token {
            token: ability.token,
            index: ability.index,
        }
    }
}

/// What a trigger aims at, chosen as it goes on the stack (CR 603.3d).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Aim {
    /// A player.
    Player(PlayerId),
    /// A permanent, a card or something on the stack.
    Object(ObjectId),
}

/// One ability waiting to go on the stack, as far as it is public.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Waiting {
    /// What the player's teaching is filed under; `None` for an ability
    /// nothing names (an emblem's, or one a continuous effect granted).
    pub key: Option<TriggerKey>,
    /// The object it came from, in this question only.
    pub source: Option<ObjectId>,
    /// What it aims at.
    pub aims: Vec<Aim>,
    /// What it will do.
    pub facts: Facts,
}

impl Waiting {
    /// Whether nothing tells these two apart, so swapping them changes
    /// nothing a player could care about.
    fn same_as(&self, other: &Self) -> bool {
        self.key.is_some()
            && self.key == other.key
            && self.aims == other.aims
            && self.facts == other.facts
    }
}

/// One player at the table, as public as the seat plate.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Standing {
    /// Who.
    pub seat: PlayerId,
    /// Life total.
    pub life: i32,
    /// Poison counters.
    pub poison: u32,
    /// Cards in the library.
    pub library: u32,
    /// On the asking player's side: the player or a teammate.
    pub ally: bool,
}

/// The table the question is asked at.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Table {
    /// The player ordering the triggers.
    pub me: PlayerId,
    /// Everyone still in the game, the asking player too.
    pub players: Vec<Standing>,
}

/// Why the suggestion stands as it does: the one line the sheet says.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reason {
    /// The order wins the game.
    Wins,
    /// The order keeps the player from losing.
    Survives,
    /// The player sorted these this way before.
    Remembered,
    /// What another trigger counts comes first.
    CountedLater,
    /// A shuffle comes before looking at the library it shuffles.
    ShuffleFirst,
    /// Looking at the library comes before drawing from it.
    LookFirst,
    /// What shows more comes before what asks for a decision.
    InformFirst,
    /// A trigger that takes its own source away comes after the source's others.
    SourceStays,
    /// Removal comes before a boost.
    RemovalFirst,
    /// The opponent is hit before the player is.
    OpponentFirst,
    /// Life is gained before it is lost.
    GainFirst,
    /// Nothing says one order is better than another.
    NoOpinion,
    /// Nothing tells these apart: every order is the same.
    NoDifference,
}

impl Reason {
    /// The line, in the player's language.
    #[must_use]
    pub const fn phrase(self) -> Phrase {
        match self {
            Self::Wins => Phrase::TriggerOrderWins,
            Self::Survives => Phrase::TriggerOrderSurvives,
            Self::Remembered => Phrase::TriggerOrderRemembered,
            Self::CountedLater => Phrase::TriggerOrderCountedLater,
            Self::ShuffleFirst => Phrase::TriggerOrderShuffleFirst,
            Self::LookFirst => Phrase::TriggerOrderLookFirst,
            Self::InformFirst => Phrase::TriggerOrderInformFirst,
            Self::SourceStays => Phrase::TriggerOrderSourceStays,
            Self::RemovalFirst => Phrase::TriggerOrderRemovalFirst,
            Self::OpponentFirst => Phrase::TriggerOrderOpponentFirst,
            Self::GainFirst => Phrase::TriggerOrderGainFirst,
            Self::NoOpinion => Phrase::TriggerOrderNoOpinion,
            Self::NoDifference => Phrase::TriggerOrderNoDifference,
        }
    }
}

/// What the player is shown, to confirm or change.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Suggestion {
    /// Indices into the question's list, the first to resolve first.
    pub order: Vec<usize>,
    /// The line beside it.
    pub reason: Reason,
    /// By the question's index: whether the player's teaching placed it.
    pub remembered: Vec<bool>,
    /// Whether the player's teaching answers this whole question: every two
    /// abilities that differ are a pair it knows, and nothing outweighed it.
    pub all_known: bool,
}

impl Suggestion {
    /// Whether this question needs no asking: the player said so in the
    /// settings, and has answered every part of it before.
    #[must_use]
    pub const fn goes_unasked(&self, memory: &TriggerMemory) -> bool {
        memory.accept_known && self.all_known
    }
}

/// The suggestion for these triggers at this table.
#[must_use]
pub fn suggest(waiting: &[Waiting], table: &Table, memory: &TriggerMemory) -> Suggestion {
    let mut plan = Plan::new(waiting.len());
    let mut reason = None;
    let mut outweighed = false;

    if let Some(found) = outcome::decide(waiting, table) {
        for pair in found.order.windows(2) {
            plan.add(pair[0], pair[1]);
        }
        reason = Some(found.reason);
    }

    let mut remembered = vec![false; waiting.len()];
    for (first, then) in memory.pairs_newest_first() {
        for a in instances(waiting, first) {
            for b in instances(waiting, then) {
                match plan.add(a, b) {
                    Added::New | Added::Already => {
                        remembered[a] = true;
                        remembered[b] = true;
                        reason.get_or_insert(Reason::Remembered);
                    }
                    Added::Refused => outweighed = true,
                }
            }
        }
    }

    for rule in rules::RULES {
        for (a, first) in waiting.iter().enumerate() {
            for (b, then) in waiting.iter().enumerate() {
                if a != b
                    && (rule.first)(first, then)
                    && !(rule.first)(then, first)
                    && plan.add(a, b) == Added::New
                {
                    reason.get_or_insert(rule.reason);
                }
            }
        }
    }

    let reason = reason.unwrap_or_else(|| {
        if all_alike(waiting) {
            Reason::NoDifference
        } else {
            Reason::NoOpinion
        }
    });
    Suggestion {
        order: plan.order(),
        reason,
        remembered,
        all_known: !outweighed && all_known(waiting, memory),
    }
}

/// The index of every instance of an ability in this question.
fn instances(waiting: &[Waiting], key: TriggerKey) -> impl Iterator<Item = usize> {
    waiting
        .iter()
        .enumerate()
        .filter(move |(_, trigger)| trigger.key == Some(key))
        .map(|(index, _)| index)
}

/// Whether every two triggers here are the same.
fn all_alike(waiting: &[Waiting]) -> bool {
    waiting.windows(2).all(|pair| pair[0].same_as(&pair[1]))
}

/// Whether the player's teaching says something about every two triggers
/// that differ, and there is at least one such pair.
fn all_known(waiting: &[Waiting], memory: &TriggerMemory) -> bool {
    let mut any = false;
    for (index, first) in waiting.iter().enumerate() {
        for then in &waiting[index + 1..] {
            if first.same_as(then) {
                continue;
            }
            let (Some(a), Some(b)) = (first.key, then.key) else {
                return false;
            };
            if a == b || memory.knows(a, b).is_none() {
                return false;
            }
            any = true;
        }
    }
    any
}
