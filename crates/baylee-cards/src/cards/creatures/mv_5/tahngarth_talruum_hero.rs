//! Tahngarth, Talruum Hero — {3}{R}{R} — Legendary Creature — Minotaur Warrior
//! Oracle: Vigilance
//! Oracle: {1}{R}, {T}: Tahngarth deals damage equal to its power to target creature. That creature deals damage equal to its power to Tahngarth.
//! Set: PLS #74 — Planeshift | Scryfall ID: c1778f37-af01-4f8c-ab9d-a4c60abf7e78 | Oracle ID: c5b4131e-13aa-4a22-b8f5-e5f9d366cb4f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TAHNGARTH_TALRUUM_HERO,
    oracle_id = "c5b4131e-13aa-4a22-b8f5-e5f9d366cb4f",
    scryfall_id = "c1778f37-af01-4f8c-ab9d-a4c60abf7e78",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Tahngarth, Talruum Hero",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::MINOTAUR, subtypes::creature::WARRIOR],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
