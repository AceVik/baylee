//! Fire Ants — {2}{R} — Creature — Insect
//! Oracle: {T}: This creature deals 1 damage to each other creature without flying.
//! Set: USG #187 — Urza's Saga | Scryfall ID: ae2e38af-a2e5-4d52-870e-1b6e0cb33cef | Oracle ID: c23543b7-f637-4a7c-9b72-ec411cd5488e
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FIRE_ANTS,
    oracle_id = "c23543b7-f637-4a7c-9b72-ec411cd5488e",
    scryfall_id = "ae2e38af-a2e5-4d52-870e-1b6e0cb33cef",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fire Ants",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INSECT],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::DealDamageEach {
            amount: Amount::Fixed(1),
            filter: &Filter::And(&[
                Filter::CREATURE,
                Filter::Another,
                Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING)),
            ]),
        }],
    )],
);
