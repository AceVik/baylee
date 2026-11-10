//! Jackalope Herd — {3}{G} — Creature — Rabbit Beast
//! Oracle: When you cast a spell, return this creature to its owner's hand.
//! Set: EXO #111 — Exodus | Scryfall ID: cb80105c-d2c0-4f8c-9302-5e6152a60f54 | Oracle ID: bcc8ce61-a881-42be-ab2c-64351d17f9f8
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::JACKALOPE_HERD,
    oracle_id = "bcc8ce61-a881-42be-ab2c-64351d17f9f8",
    scryfall_id = "cb80105c-d2c0-4f8c-9302-5e6152a60f54",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Jackalope Herd",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::RABBIT, subtypes::creature::BEAST],
        power = Some(4),
        toughness = Some(5),
    ),],
    abilities = &[triggered!(
        Trigger::SpellCast(&Filter::ControlledByYou),
        &[Effect::bounce(TargetSpec::ThisObject)]
    ),],
);
