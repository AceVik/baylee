//! Tracker — {2}{G} — Creature — Human
//! Oracle: {G}{G}, {T}: This creature deals damage equal to its power to target creature. That creature deals damage equal to its power to this creature.
//! Set: ME3 #136 — Masters Edition III | Scryfall ID: 74f0400c-35dd-4ba5-97d1-2e57689c9d53 | Oracle ID: 86260d53-a68e-43cb-88ba-6a97c0b2f4f4

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TRACKER,
    oracle_id = "86260d53-a68e-43cb-88ba-6a97c0b2f4f4",
    scryfall_id = "74f0400c-35dd-4ba5-97d1-2e57689c9d53",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tracker",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{G}{G}", TapSelf),
        &[
            Effect::DamageEqualToPower {
                dealer: TargetSlot::This,
                to: TargetSlot::First,
            },
            Effect::DamageEqualToPower {
                dealer: TargetSlot::First,
                to: TargetSlot::This,
            },
        ],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
