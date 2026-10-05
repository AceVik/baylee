//! Divine Intervention — {6}{W}{W} — Enchantment
//! Oracle: This enchantment enters with two intervention counters on it.
//! Oracle: At the beginning of your upkeep, remove an intervention counter from this enchantment.
//! Oracle: When you remove the last intervention counter from this enchantment, the game is a draw.
//! Set: ME3 #8 — Masters Edition III | Scryfall ID: e250897e-a875-4c3c-bfec-4a09143dd587 | Oracle ID: 0158a440-6573-4f12-958b-a5f0cf190d5b
// PARTIAL — every printed sentence is off the card: the intervention counter
// is not an assigned `counters` id, and nothing draws the game.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DIVINE_INTERVENTION,
    oracle_id = "0158a440-6573-4f12-958b-a5f0cf190d5b",
    scryfall_id = "e250897e-a875-4c3c-bfec-4a09143dd587",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "the intervention counter is not in baylee_cards_dsl::counters and no \
         Effect makes the game a draw"
    ),
    faces = &[face!(
        name = "Divine Intervention",
        mana_cost = mana!("{6}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// NOT SUPPORTED: "This enchantment enters with two intervention counters on
// it." and "At the beginning of your upkeep, remove an intervention counter
// from this enchantment." — no `counters::INTERVENTION` id is assigned, and a
// card file may not spell `CounterKind::Custom` as a number.
// NOT SUPPORTED: "When you remove the last intervention counter from this
// enchantment, the game is a draw." — no Effect draws the game;
// `Effect::LoseGame` makes only the ability's controller lose.
