//! Tranquil Grove — {1}{G} — Enchantment
//! Oracle: {1}{G}{G}: Destroy all other enchantments.
//! Set: 6ED #258 — Classic Sixth Edition | Scryfall ID: 1177118a-6455-4177-8a6d-a43a03160ab3 | Oracle ID: 2f2a3588-b9a6-4c1a-89f8-b11339851b8f
// IMPLEMENTED — {1}{G}{G}: Destroy all other enchantments.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TRANQUIL_GROVE,
    oracle_id = "2f2a3588-b9a6-4c1a-89f8-b11339851b8f",
    scryfall_id = "1177118a-6455-4177-8a6d-a43a03160ab3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tranquil Grove",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{1}{G}{G}"),
        &[Effect::destroy_all(&Filter::And(&[
            Filter::ENCHANTMENT,
            Filter::Another,
        ]))]
    ),],
);
