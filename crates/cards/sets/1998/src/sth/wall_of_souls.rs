//! Wall of Souls — {1}{B} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: Whenever this creature is dealt combat damage, it deals that much damage to target opponent or planeswalker.
//! Set: TPR #123 — Tempest Remastered | Scryfall ID: a5b8d9a2-91d7-4172-b778-b5dc3ac28abe | Oracle ID: 2c85c9ea-c3fd-4981-a65c-4405bd87764e
// PARTIAL — The combat-only received-damage trigger is not supported; only defender and the creature body are implemented

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_SOULS,
    oracle_id = "2c85c9ea-c3fd-4981-a65c-4405bd87764e",
    scryfall_id = "a5b8d9a2-91d7-4172-b778-b5dc3ac28abe",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "The combat-only received-damage trigger is not supported; only defender and the creature body are implemented"
    ),
    keywords = KeywordSet::DEFENDER,
    faces = &[face!(
        name = "Wall of Souls",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(4),
    ),],
    // NOT SUPPORTED: The combat-only received-damage trigger is not supported; only defender and the creature body are implemented.
);
