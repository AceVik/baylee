//! Righteous Aura — {1}{W} — Enchantment
//! Oracle: {W}, Pay 2 life: The next time a source of your choice would deal damage to you this turn, prevent that damage.
//! Set: MMQ #45 — Mercadian Masques | Scryfall ID: 66d3bcb4-6cbd-4144-a95d-f61e68c10296 | Oracle ID: 5bf93d4f-be59-4aa7-83d5-7c35d1f07fae
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RIGHTEOUS_AURA,
    oracle_id = "5bf93d4f-be59-4aa7-83d5-7c35d1f07fae",
    scryfall_id = "66d3bcb4-6cbd-4144-a95d-f61e68c10296",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Righteous Aura",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{W}", PayLife(2)),
        &[Effect::PreventNextFromChosenSource {
            sources: &Filter::Any,
            combat_only: false,
            all_but: 0,
            gain_life: false,
        }],
    )],
);
