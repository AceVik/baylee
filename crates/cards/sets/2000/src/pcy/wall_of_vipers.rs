//! Wall of Vipers — {2}{B} — Creature — Snake Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: {3}: Destroy this creature and target creature it's blocking. Any player may activate this ability.
//! Set: PCY #80 — Prophecy | Scryfall ID: 00042443-4d4e-4087-b4e5-5e781e7cc5fa | Oracle ID: 4fcfe370-ba33-438f-bd96-2ae560f59df9
// IMPLEMENTED — defender; any player may pay {3} to destroy it and the creature it's blocking.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_VIPERS,
    oracle_id = "4fcfe370-ba33-438f-bd96-2ae560f59df9",
    scryfall_id = "00042443-4d4e-4087-b4e5-5e781e7cc5fa",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::DEFENDER,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Wall of Vipers",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SNAKE, subtypes::creature::WALL],
        power = Some(2),
        toughness = Some(4),
    ),],
    abilities = &[activated!(
        cost!("{3}"),
        &[
            Effect::destroy(TargetSpec::Object(&Filter::And(&[
                Filter::CREATURE,
                Filter::BlockedBySource
            ]))),
            Effect::destroy(TargetSpec::ThisObject),
        ],
        zone = ActivationZone::BattlefieldAnyPlayer,
        target = Some(TargetSpec::Object(&Filter::And(&[
            Filter::CREATURE,
            Filter::BlockedBySource
        ]))),
    )],
);
