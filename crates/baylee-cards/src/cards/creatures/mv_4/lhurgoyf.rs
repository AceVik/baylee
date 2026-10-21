//! Lhurgoyf — {2}{G}{G} — Creature — Lhurgoyf
//! Oracle: Lhurgoyf's power is equal to the number of creature cards in all graveyards and its toughness is equal to that number plus 1.
//! Set: CM2 #141 — Commander Anthology Volume II | Scryfall ID: ea5d550a-bd8b-4f9e-ac7d-c35ebbe313e6 | Oracle ID: 0fe76004-b2f4-4b11-abce-75fd76f68b4d
// NOT SUPPORTED: characteristic-defining P/T counting creature cards across all graveyards is not supported; plays as a 0/1 creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LHURGOYF,
    oracle_id = "0fe76004-b2f4-4b11-abce-75fd76f68b4d",
    scryfall_id = "ea5d550a-bd8b-4f9e-ac7d-c35ebbe313e6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "characteristic-defining P/T counting creature cards across all graveyards is not supported; plays as a 0/1 creature",
    ),
    faces = &[face!(
        name = "Lhurgoyf",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LHURGOYF],
        power = Some(0),
        toughness = Some(1),
    ),],
);
