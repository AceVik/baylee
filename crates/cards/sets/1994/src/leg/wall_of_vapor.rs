//! Wall of Vapor — {3}{U} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: Prevent all damage that would be dealt to this creature by creatures it's blocking.
//! Set: CHR #27 — Chronicles | Scryfall ID: 309c1b2a-0230-4b66-84a0-32b8cd6d31eb | Oracle ID: 7d08d128-863c-4cca-837f-847ff44acef5

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_VAPOR,
    oracle_id = "7d08d128-863c-4cca-837f-847ff44acef5",
    scryfall_id = "309c1b2a-0230-4b66-84a0-32b8cd6d31eb",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::DEFENDER,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Wall of Vapor",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(1),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::PreventDamageFrom(&Filter::BlockedBySource)
    )],
);
