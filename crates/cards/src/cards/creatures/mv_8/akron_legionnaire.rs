//! Akron Legionnaire — {6}{W}{W} — Creature — Giant Soldier
//! Oracle: Except for creatures named Akron Legionnaire and artifact creatures, creatures you control can't attack.
//! Set: ME3 #1 — Masters Edition III | Scryfall ID: b3e10240-8a4f-410f-a9b2-5df89d7038df | Oracle ID: c5f50a78-9dc4-4f3c-aa11-b41c8278bc16
// IMPLEMENTED — creatures you control can't attack except Akron Legionnaires and artifact creatures.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AKRON_LEGIONNAIRE,
    oracle_id = "c5f50a78-9dc4-4f3c-aa11-b41c8278bc16",
    scryfall_id = "b3e10240-8a4f-410f-a9b2-5df89d7038df",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Akron Legionnaire",
        mana_cost = mana!("{6}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GIANT, subtypes::creature::SOLDIER],
        power = Some(8),
        toughness = Some(4),
    ),],
    abilities = &[static_ability!(
        Filter::And(&[
            Filter::CREATURE,
            Filter::ControlledByYou,
            Filter::Not(&Filter::ARTIFACT),
            Filter::Not(&Filter::Named("Akron Legionnaire"))
        ]),
        Modifier::AddKeyword(KeywordSet::CANT_ATTACK)
    )],
);
