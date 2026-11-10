//! Amok — {1}{R} — Enchantment
//! Oracle: {1}, Discard a card at random: Put a +1/+1 counter on target creature.
//! Set: STH #76 — Stronghold | Scryfall ID: 5c5bdb8c-2a2e-47cf-a502-2d62f9ada3fa | Oracle ID: 2ddbbe65-f928-4b8e-8c4c-a9ce82e2594d
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AMOK,
    oracle_id = "2ddbbe65-f928-4b8e-8c4c-a9ce82e2594d",
    scryfall_id = "5c5bdb8c-2a2e-47cf-a502-2d62f9ada3fa",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Amok",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{1}", DiscardRandom),
        &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1)
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE))
    ),],
);
