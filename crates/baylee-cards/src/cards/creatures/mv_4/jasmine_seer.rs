//! Jasmine Seer — {3}{W} — Creature — Human Wizard
//! Oracle: {2}{W}, {T}: Reveal any number of white cards in your hand. You gain 2 life for each card revealed this way.
//! Set: UDS #10 — Urza's Destiny | Scryfall ID: a6641dd2-5b9c-4089-8a71-a3a1a9c29f8b | Oracle ID: d934a7f0-eb46-4a2c-b0fd-e3071e23da4a
// NOT SUPPORTED: variable-count hand reveal (reveal any number of white cards from your hand and gain 2 life per card revealed) is unsupported; plays as a 1/1 Human Wizard body
// PARTIAL: {3}{W} 1/1 Human Wizard creature body implemented; missing "{2}{W}, {T}: Reveal any number of white cards in your hand. You gain 2 life for each card revealed this way" because the DSL has no cost or effect to reveal any number of matching cards from your hand and count them.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::JASMINE_SEER,
    oracle_id = "d934a7f0-eb46-4a2c-b0fd-e3071e23da4a",
    scryfall_id = "a6641dd2-5b9c-4089-8a71-a3a1a9c29f8b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "variable-count hand reveal (reveal any number of white cards from your hand and gain 2 life per card revealed) is unsupported; plays as a 1/1 Human Wizard body"
    ),
    faces = &[face!(
        name = "Jasmine Seer",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);
