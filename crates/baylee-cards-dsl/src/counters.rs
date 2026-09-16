//! The counters Magic prints that carry no rule of their own.
//!
//! [`CounterKind`] names eleven counters because eleven of them mean
//! something to the rules: `P1P1` and `M1M1` move power and toughness
//! (CR 122.1a), `Loyalty` pays for planeswalker abilities (CR 122.1e),
//! `Lore` advances a Saga, `Time` is suspend and vanishing,
//! `Poison`/`Energy`/`Rad` sit on players (CR 122.1f, CR 122.1i),
//! `Lifelink` grants a keyword (CR 122.1b) and `Level` gates a Class
//! (CR 716). An engine that did not know those would play the card wrong.
//!
//! A great many other counters are **a name and a number**. A depletion
//! counter does nothing at all; the card that prints it says what happens
//! when it runs out, and the rules have never heard of it. Counted over
//! every `//! Oracle:` header on 2026-09-16, this pool's 1536 cards print
//! **38** distinct counter words and ten of them have a variant here, so
//! twenty-eight do not — in one pool, before any set ships. Giving each one
//! a variant would grow the enum, and with it the wire-stable enum in
//! `baylee-view` that [`crate::CounterKind`] is projected onto, to teach
//! the engine nothing.
//!
//! **A word that earns a rule earns a variant**, and that line is not
//! decorative: shield, stun and finality counters are in those twenty-eight
//! and each has a rule of its own (CR 122.1c, CR 122.1d, CR 122.1h), so
//! each belongs in the enum when it is written and not in the table below.
//! `every_assigned_counter_is_a_custom_id` is where that is enforced.
//!
//! So they are [`CounterKind::Custom`] ids, and this module is the place the
//! ids are **assigned**. Before it there was one: Luminarch Ascension wrote
//! a bare `CounterKind::Custom(1)` for its quest counters, which is a number
//! the next card had no way to know was taken. A constant with a doc comment
//! is the same zero-cost value and says which word it stands for.
//!
//! What a client draws for one of these is still a bare dot — the view
//! carries the id and has no table to turn it into a name. That is a real
//! hole and it is named in `docs/engine-gaps.md`; the answer is a counter
//! name table in `GameStatic`, not a variant per word.

use crate::effect::CounterKind;

/// Quest counters (Luminarch Ascension).
pub const QUEST: CounterKind = CounterKind::Custom(1);

/// Depletion counters.
///
/// **Two cycles print the word**, which is the reason this constant exists
/// rather than a number per card. The five Mercadian Masques lands —
/// Hickory Woodlot, Peat Bog, Remote Farm, Sandstone Needle, Saprazzan
/// Skerry — arrive with two and sacrifice themselves at nought. The five
/// Mirage ones — Land Cap, Lava Tubes, River Delta, Timberline Ridge,
/// Veldt — *put* a depletion counter on themselves, do not untap while one
/// is there and take it off at upkeep, which is `engine-gaps.md`'s G5 and
/// is not written yet. When it is, it is this id: two ids for one printed
/// word would be two counters a Doubling Season and a proliferate could
/// tell apart and a player could not.
pub const DEPLETION: CounterKind = CounterKind::Custom(2);

/// Mining counters (Gemstone Mine).
pub const MINING: CounterKind = CounterKind::Custom(3);

/// Storage counters — the seventeen lands that bank mana a turn at a time
/// (Fountain of Cho, Mage-Ring Network, Calciform Pools, …).
pub const STORAGE: CounterKind = CounterKind::Custom(4);

/// Every id this module assigns, with the word it stands for.
///
/// It exists so the ids can be checked rather than trusted, which is the
/// whole difference between a registry and a handful of numbers that happen not to
/// clash today.
pub const ASSIGNED: &[(&str, CounterKind)] = &[
    ("quest", QUEST),
    ("depletion", DEPLETION),
    ("mining", MINING),
    ("storage", STORAGE),
];

#[cfg(test)]
mod tests {
    use super::{ASSIGNED, CounterKind};

    /// Two words sharing an id would be one counter wearing two names: a
    /// permanent carrying both would count each as the other, and nothing
    /// in the engine could tell them apart, because to the engine an id is
    /// all a custom counter is.
    #[test]
    fn no_two_counter_words_share_an_id() {
        for (i, (word, kind)) in ASSIGNED.iter().enumerate() {
            for (other_word, other_kind) in &ASSIGNED[i + 1..] {
                assert_ne!(
                    kind, other_kind,
                    "{word} and {other_word} are the same counter"
                );
            }
        }
    }

    /// And every one of them is a `Custom` id, which is what says this
    /// module is about the counters the rules have nothing to say about.
    /// A word that earns a rule earns a variant instead, and moving it
    /// would fail here.
    #[test]
    fn every_assigned_counter_is_a_custom_id() {
        for (word, kind) in ASSIGNED {
            assert!(
                matches!(kind, CounterKind::Custom(_)),
                "{word} is a named kind and does not belong in this table"
            );
        }
    }
}
