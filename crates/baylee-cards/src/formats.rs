//! What a deck of each shape must be: its size, how many copies of a card
//! it may hold, and for Commander its leaders and their colour identity.
//!
//! One pure predicate, for every reader that has to say whether a deck is
//! one: the trained AI's deck generator today, the gateway's deck validator
//! and the deckbuilder once they call it (`docs/banlists-and-formats.md`).
//! It knows shapes, not legality: no ban list, no set rotation. Who may lead
//! and which leaders may pair is [`crate::decks::leader_of`] and
//! [`crate::decks::may_lead_together`], not a second copy of that rule.

use std::collections::BTreeMap;

use baylee_core::color::ColorSet;
use baylee_core::ids::CardIndex;
use baylee_core::types::{SupertypeSet, TypeSet};

use crate::decks::{leader_of, may_lead_together};

/// A deck's shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// At least [`CONSTRUCTED_MIN`] cards, at most [`CONSTRUCTED_COPIES`] of
    /// any card but a basic land.
    Constructed,
    /// Exactly [`SINGLETON_SIZE`] cards, one of each but basic lands.
    Highlander,
    /// One or two leaders that may lead together, and with them exactly
    /// [`SINGLETON_SIZE`] cards, one of each but basic lands, every card
    /// within the leaders' colour identity.
    Commander,
}

/// The fewest cards a constructed deck holds; more is a deck too.
pub const CONSTRUCTED_MIN: usize = 60;
/// The most copies of one card a constructed deck holds, basics aside.
pub const CONSTRUCTED_COPIES: usize = 4;
/// A singleton deck's size, leaders included.
pub const SINGLETON_SIZE: usize = 100;

/// Why a deck is not of its shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    /// A card the pool has no definition for.
    UnknownCard(CardIndex),
    /// Fewer cards than the shape needs.
    TooFew {
        /// Cards in the deck, leaders included.
        have: usize,
        /// The least the shape takes.
        need: usize,
    },
    /// Not the exact size a singleton shape takes.
    WrongSize {
        /// Cards in the deck, leaders included.
        have: usize,
        /// The size the shape takes.
        need: usize,
    },
    /// More copies of a card than the shape allows.
    TooManyCopies {
        /// The card.
        card: CardIndex,
        /// Copies in the deck.
        have: usize,
        /// Copies allowed.
        limit: usize,
    },
    /// A Commander deck without a leader, or with more than two.
    Leaders {
        /// Leaders named.
        have: usize,
    },
    /// A leader that may not lead a deck.
    NotALeader(CardIndex),
    /// Two leaders that may not lead together.
    CannotLeadTogether(CardIndex, CardIndex),
    /// A card outside the leaders' colour identity.
    OutsideIdentity(CardIndex),
    /// Leaders were named for a shape that has none.
    LeadersNotTaken,
}

/// Whether `card` is a basic land, which every shape takes any number of.
#[must_use]
pub fn is_basic_land(card: CardIndex) -> bool {
    crate::by_index(card).is_some_and(|def| {
        let face = &def.faces[0];
        face.supertypes.contains(SupertypeSet::BASIC) && face.types.contains(TypeSet::LAND)
    })
}

/// A card's colour identity, as the registry records it (every face's).
#[must_use]
pub fn identity(card: CardIndex) -> Option<ColorSet> {
    crate::by_index(card).map(|def| def.color_identity)
}

/// Everything that keeps `main` (the library, leaders not in it) and
/// `leaders` from being a deck of `shape`; empty when it is one.
#[must_use]
pub fn check(shape: Shape, main: &[CardIndex], leaders: &[CardIndex]) -> Vec<Problem> {
    let mut problems = Vec::new();
    for card in main.iter().chain(leaders) {
        if crate::by_index(*card).is_none() {
            problems.push(Problem::UnknownCard(*card));
        }
    }
    let size = main.len() + leaders.len();
    let mut copies: BTreeMap<CardIndex, usize> = BTreeMap::new();
    for card in main.iter().chain(leaders) {
        *copies.entry(*card).or_default() += 1;
    }
    let limit = match shape {
        Shape::Constructed => CONSTRUCTED_COPIES,
        Shape::Highlander | Shape::Commander => 1,
    };
    for (card, have) in &copies {
        if *have > limit && !is_basic_land(*card) {
            problems.push(Problem::TooManyCopies {
                card: *card,
                have: *have,
                limit,
            });
        }
    }
    match shape {
        Shape::Constructed => {
            if size < CONSTRUCTED_MIN {
                problems.push(Problem::TooFew {
                    have: size,
                    need: CONSTRUCTED_MIN,
                });
            }
        }
        Shape::Highlander | Shape::Commander => {
            if size != SINGLETON_SIZE {
                problems.push(Problem::WrongSize {
                    have: size,
                    need: SINGLETON_SIZE,
                });
            }
        }
    }
    if shape != Shape::Commander {
        if !leaders.is_empty() {
            problems.push(Problem::LeadersNotTaken);
        }
        return problems;
    }
    if leaders.is_empty() || leaders.len() > 2 {
        problems.push(Problem::Leaders {
            have: leaders.len(),
        });
    }
    let led: Vec<_> = leaders.iter().filter_map(|l| leader_of(*l)).collect();
    for l in &led {
        if !l.eligible {
            problems.push(Problem::NotALeader(l.index));
        }
    }
    if let [a, b] = led.as_slice()
        && a.eligible
        && b.eligible
        && !may_lead_together(a, b)
    {
        problems.push(Problem::CannotLeadTogether(a.index, b.index));
    }
    let allowed = leaders
        .iter()
        .filter_map(|l| identity(*l))
        .fold(ColorSet::EMPTY, ColorSet::union);
    // Without a leader there is no identity to keep to; the missing leader
    // is the problem, not every card that has a colour.
    let mut outside: Vec<CardIndex> = main
        .iter()
        .filter(|_| !leaders.is_empty())
        .filter(|c| identity(**c).is_some_and(|id| !id.difference(allowed).is_empty()))
        .copied()
        .collect();
    outside.sort();
    outside.dedup();
    problems.extend(outside.into_iter().map(Problem::OutsideIdentity));
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(name: &str) -> CardIndex {
        crate::decks::by_name(name).unwrap_or_else(|| panic!("{name} is in the pool"))
    }

    fn many(name: &str, n: usize) -> Vec<CardIndex> {
        vec![card(name); n]
    }

    #[test]
    fn a_constructed_deck_takes_four_copies_and_any_number_of_basics() {
        let mut deck = many("Island", 52);
        deck.extend(many("Counterspell", 4));
        deck.extend(many("Sol Ring", 4));
        assert_eq!(check(Shape::Constructed, &deck, &[]), []);
        deck.push(card("Sol Ring"));
        assert_eq!(
            check(Shape::Constructed, &deck, &[]),
            [Problem::TooManyCopies {
                card: card("Sol Ring"),
                have: 5,
                limit: 4
            }]
        );
        // More than sixty is a deck; fewer is not.
        let short = many("Island", 59);
        assert_eq!(
            check(Shape::Constructed, &short, &[]),
            [Problem::TooFew { have: 59, need: 60 }]
        );
        assert_eq!(check(Shape::Constructed, &many("Island", 75), &[]), []);
    }

    #[test]
    fn a_highlander_deck_is_a_hundred_singletons_and_basics() {
        let mut deck = many("Plains", 98);
        deck.push(card("Sol Ring"));
        deck.push(card("Swords to Plowshares"));
        assert_eq!(check(Shape::Highlander, &deck, &[]), []);
        deck[0] = card("Sol Ring");
        assert!(
            check(Shape::Highlander, &deck, &[]).contains(&Problem::TooManyCopies {
                card: card("Sol Ring"),
                have: 2,
                limit: 1
            })
        );
        assert_eq!(
            check(Shape::Highlander, &deck[..99], &[]),
            [
                Problem::TooManyCopies {
                    card: card("Sol Ring"),
                    have: 2,
                    limit: 1
                },
                Problem::WrongSize {
                    have: 99,
                    need: 100
                }
            ]
        );
    }

    /// Every card within the leader's identity, the leader counted among
    /// the hundred, a second leader only when the two may lead together.
    #[test]
    fn a_commander_deck_keeps_to_its_leaders() {
        let tazri = card("General Tazri");
        let mut deck = many("Island", 97);
        deck.push(card("Sol Ring"));
        deck.push(card("Swords to Plowshares"));
        assert_eq!(check(Shape::Commander, &deck, &[tazri]), []);
        assert_eq!(
            check(Shape::Commander, &deck, &[]),
            [
                Problem::WrongSize {
                    have: 99,
                    need: 100
                },
                Problem::Leaders { have: 0 }
            ]
        );
        // A mono-white leader cannot take an Island.
        let white = crate::all()
            .find(|d| {
                crate::decks::leader_of(d.index).is_some_and(|l| l.eligible)
                    && d.color_identity == ColorSet::from_slice(&[baylee_core::color::Color::White])
            })
            .expect("a mono-white leader in the pool")
            .index;
        assert!(
            check(Shape::Commander, &deck, &[white])
                .contains(&Problem::OutsideIdentity(card("Island")))
        );
        // Two leaders that are not partners.
        let problems = check(Shape::Commander, &deck[1..], &[tazri, white]);
        assert!(
            problems.contains(&Problem::CannotLeadTogether(tazri, white)),
            "{problems:?}"
        );
        // A card that may not lead.
        assert!(
            check(Shape::Commander, &deck, &[card("Sol Ring")])
                .contains(&Problem::NotALeader(card("Sol Ring")))
        );
    }

    #[test]
    fn only_commander_takes_leaders() {
        let deck = many("Island", 99);
        assert!(
            check(Shape::Highlander, &deck, &[card("General Tazri")])
                .contains(&Problem::LeadersNotTaken)
        );
    }
}
