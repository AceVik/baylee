//! Verdeloth the Ancient — {4}{G}{G} — Legendary Creature — Treefolk
//! Oracle: Kicker {X} (You may pay an additional {X} as you cast this spell.)
//! Oracle: Saproling creatures and other Treefolk creatures get +1/+1.
//! Oracle: When Verdeloth enters, if it was kicked, create X 1/1 green Saproling creature tokens.
//! Set: CMM #330 — Commander Masters | Scryfall ID: 980b31eb-a9cd-44d2-8755-9323bb4de2f0 | Oracle ID: 8222321f-b74d-4f74-9ee7-79455433345a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VERDELOTH_THE_ANCIENT,
    oracle_id = "8222321f-b74d-4f74-9ee7-79455433345a",
    scryfall_id = "980b31eb-a9cd-44d2-8755-9323bb4de2f0",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Verdeloth the Ancient",
        mana_cost = mana!("{4}{G}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::TREEFOLK],
        power = Some(4),
        toughness = Some(7),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
