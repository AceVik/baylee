//! Mirari's Wake — {3}{G}{W} — Enchantment
//! Oracle: Creatures you control get +1/+1.
//! Oracle: Whenever you tap a land for mana, add one mana of any type that land produced.
//! Set: CMM #928 — Commander Masters | Scryfall ID: ef4fa4e8-39e8-47a6-b975-ff5d46e59edd | Oracle ID: 852657c0-18a4-4b28-b9ae-7728acdb5044
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MIRARI_S_WAKE,
    oracle_id = "852657c0-18a4-4b28-b9ae-7728acdb5044",
    scryfall_id = "ef4fa4e8-39e8-47a6-b975-ff5d46e59edd",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(
        name = "Mirari's Wake",
        mana_cost = mana!("{3}{G}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
