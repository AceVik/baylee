//! Good play, for two triggers at a time: what a careful player would do
//! when the game's end does not decide it.
//!
//! The list is in order of weight, and a rule never undoes a heavier one
//! (`plan`). What depends on what comes first, because getting that wrong
//! changes what a trigger does; matters of advantage follow. A rule says only
//! what it knows: two triggers it has no opinion on stay as they came.
//!
//! A trigger that sets others off is read with them
//! ([`Facts::with_followers`](super::facts::Facts::with_followers)), so the
//! same rules place it where what it sets off belongs.

use super::Reason;
use super::Waiting;
use super::facts::{Deeds, Stuff};

/// One rule of good play.
pub(super) struct Rule {
    /// What the sheet says when this is what decided.
    pub(super) reason: Reason,
    /// Whether the first trigger should resolve before the second.
    pub(super) first: fn(&Waiting, &Waiting) -> bool,
}

pub(super) const RULES: &[Rule] = &[
    Rule {
        reason: Reason::CountedLater,
        first: counted_later,
    },
    Rule {
        reason: Reason::ShuffleFirst,
        first: shuffle_first,
    },
    Rule {
        reason: Reason::LookFirst,
        first: look_first,
    },
    Rule {
        reason: Reason::InformFirst,
        first: inform_first,
    },
    Rule {
        reason: Reason::SourceStays,
        first: source_stays,
    },
    Rule {
        reason: Reason::RemovalFirst,
        first: removal_first,
    },
    Rule {
        reason: Reason::OpponentFirst,
        first: opponent_first,
    },
    Rule {
        reason: Reason::GainFirst,
        first: gain_first,
    },
];

/// A counter on a permanent means something to that permanent's own count.
fn same_source(a: &Waiting, b: &Waiting) -> bool {
    a.source.is_some() && a.source == b.source
}

/// What the second one counts, the first one makes: counters before what
/// counts them, tokens before a count of creatures, mana before a payment.
fn counted_later(a: &Waiting, b: &Waiting) -> bool {
    a.facts.makes.meets(b.facts.reads, Stuff::ANYWHERE)
        || (same_source(a, b) && a.facts.makes.meets(b.facts.reads, Stuff::OWN_COUNTERS))
}

/// Looking at a library and then shuffling it is looking for nothing.
fn shuffle_first(a: &Waiting, b: &Waiting) -> bool {
    a.facts.deeds.has(Deeds::SHUFFLES) && b.facts.deeds.has(Deeds::LOOKS)
}

/// A scry works on the card about to be drawn.
fn look_first(a: &Waiting, b: &Waiting) -> bool {
    a.facts.deeds.has(Deeds::LOOKS) && b.facts.deeds.has(Deeds::DRAWS)
}

/// Knowing more before having to choose: cards in hand, a sorted library, a
/// search, ahead of a "may", a payment, a discard or a sacrifice.
fn inform_first(a: &Waiting, b: &Waiting) -> bool {
    a.facts.deeds.has(Deeds::INFORMS)
        && b.facts.deeds.has(Deeds::ASKS)
        && !a.facts.deeds.has(Deeds::ASKS)
}

/// A trigger that takes its own source away waits for the source's others.
fn source_stays(a: &Waiting, b: &Waiting) -> bool {
    same_source(a, b) && b.facts.deeds.has(Deeds::LEAVES) && !a.facts.deeds.has(Deeds::LEAVES)
}

/// A boost is spent on the board that is left.
fn removal_first(a: &Waiting, b: &Waiting) -> bool {
    a.facts.deeds.has(Deeds::REMOVES) && b.facts.deeds.has(Deeds::PUMPS)
}

/// Whoever is hit first may be the one who cannot answer.
fn opponent_first(a: &Waiting, b: &Waiting) -> bool {
    a.facts.deeds.has(Deeds::HURTS_OPPONENT)
        && !a.facts.deeds.has(Deeds::HURTS_ME)
        && b.facts.deeds.has(Deeds::HURTS_ME)
}

/// Life gained first is life that can be lost.
fn gain_first(a: &Waiting, b: &Waiting) -> bool {
    a.facts.deeds.has(Deeds::HEALS_ME) && b.facts.deeds.has(Deeds::HURTS_ME)
}
