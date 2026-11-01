//! Emperor Crocodile — {3}{G} — Creature — Crocodile
//! Oracle: When you control no other creatures, sacrifice this creature.
//! Set: EMA #165 — Eternal Masters | Scryfall ID: 6c8c30e2-e751-4481-9a46-413c52bbff90 | Oracle ID: 26eb7ee3-c0b6-4be0-bc80-92952f55a6f9
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static NO_OTHER_CREATURES: Condition = Condition::BattlefieldCountAtMost(
    &Filter::And(&[Filter::YOUR_CREATURE, Filter::Not(&Filter::This)]),
    0,
);

card!(
    index = index::EMPEROR_CROCODILE,
    oracle_id = "26eb7ee3-c0b6-4be0-bc80-92952f55a6f9",
    scryfall_id = "6c8c30e2-e751-4481-9a46-413c52bbff90",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Emperor Crocodile",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CROCODILE],
        power = Some(5),
        toughness = Some(5),
    ),],
    abilities = &[triggered!(
        Trigger::State(&NO_OTHER_CREATURES),
        &[Effect::SacrificeSelf],
    )],
);
