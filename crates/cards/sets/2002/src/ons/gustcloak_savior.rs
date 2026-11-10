//! Gustcloak Savior — {4}{W} — Creature — Bird Soldier
//! Oracle: Flying
//! Oracle: Whenever a creature you control becomes blocked, you may untap that creature and remove it from combat.
//! Set: DDO #11 — Duel Decks: Elspeth vs. Kiora | Scryfall ID: 5303ce82-e82a-498e-af4f-16a56d8188ff | Oracle ID: 3d5205ad-6fd0-4afc-b817-c86b6bddb325
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GUSTCLOAK_SAVIOR,
    oracle_id = "3d5205ad-6fd0-4afc-b817-c86b6bddb325",
    scryfall_id = "5303ce82-e82a-498e-af4f-16a56d8188ff",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Gustcloak Savior",
        mana_cost = mana!("{4}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD, subtypes::creature::SOLDIER],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
