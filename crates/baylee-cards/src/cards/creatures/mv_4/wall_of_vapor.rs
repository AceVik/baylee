//! Wall of Vapor — {3}{U} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: Prevent all damage that would be dealt to this creature by creatures it's blocking.
//! Set: CHR #27 — Chronicles | Scryfall ID: 309c1b2a-0230-4b66-84a0-32b8cd6d31eb | Oracle ID: 7d08d128-863c-4cca-837f-847ff44acef5
// PARTIAL — defender is a keyword bit; the prevention is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_VAPOR,
    oracle_id = "7d08d128-863c-4cca-837f-847ff44acef5",
    scryfall_id = "309c1b2a-0230-4b66-84a0-32b8cd6d31eb",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::DEFENDER,
    coverage = Coverage::Partial(
        "no Modifier prevents damage from a filtered source: \
         PreventDamageToIt is combat-only and names no source filter"
    ),
    faces = &[face!(
        name = "Wall of Vapor",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(1),
    ),],
    // NOT SUPPORTED: "Prevent all damage that would be dealt to this creature
    // by creatures it's blocking." — `Modifier::PreventDamageToIt` prevents
    // combat damage from every source and takes no source filter, so it
    // cannot say the damage must come from a creature this Wall is blocking
    // and would still let such a creature's noncombat damage through;
    // `Modifier::ProtectionFrom` would prevent the damage but also stops the
    // creatures from targeting and blocking, which the card does not print.
    abilities = &[],
);
