//! Lhurgoyf — {2}{G}{G} — Creature — Lhurgoyf
//! Oracle: Lhurgoyf's power is equal to the number of creature cards in all graveyards and its toughness is equal to that number plus 1.
//! Set: CM2 #141 — Commander Anthology Volume II | Scryfall ID: ea5d550a-bd8b-4f9e-ac7d-c35ebbe313e6 | Oracle ID: 0fe76004-b2f4-4b11-abce-75fd76f68b4d

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LHURGOYF,
    oracle_id = "0fe76004-b2f4-4b11-abce-75fd76f68b4d",
    scryfall_id = "ea5d550a-bd8b-4f9e-ac7d-c35ebbe313e6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Lhurgoyf",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LHURGOYF],
        power = Some(0),
        toughness = Some(1),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CharacteristicPT {
            count: PtCount::InAllGraveyards(&Filter::CREATURE),
            toughness_plus: 1,
        }
    )],
);
