//! Volrath's Gardens — {1}{G} — Enchantment
//! Oracle: {2}, Tap an untapped creature you control: You gain 2 life. Activate only as a sorcery.
//! Set: STH #124 — Stronghold | Scryfall ID: 7d74f7d8-10eb-4082-9b07-41de13359b8c | Oracle ID: ff99337e-410c-43af-9475-7995d690bcef
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VOLRATH_S_GARDENS,
    oracle_id = "ff99337e-410c-43af-9475-7995d690bcef",
    scryfall_id = "7d74f7d8-10eb-4082-9b07-41de13359b8c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Volrath's Gardens",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{2}", TapOther(&Filter::YOUR_CREATURE)),
        &[Effect::gain_life(2)],
        timing = ActivationTiming::SorcerySpeed,
    ),],
);
