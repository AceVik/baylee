//! Wall of Shadows — {1}{B}{B} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: Prevent all damage that would be dealt to this creature by creatures it's blocking.
//! Oracle: This creature can't be the target of spells that can target only Walls or of abilities that can target only Walls.
//! Set: CHR #41 — Chronicles | Scryfall ID: 69c6e076-d7bf-435b-ba79-84aa9f073130 | Oracle ID: 0e6ff89e-9ae7-4dc3-98b7-05401c5df266
// PARTIAL — defender is the whole card; the two restrictions are off it.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_SHADOWS,
    oracle_id = "0e6ff89e-9ae7-4dc3-98b7-05401c5df266",
    scryfall_id = "69c6e076-d7bf-435b-ba79-84aa9f073130",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Wall of Shadows",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(1),
    ),],
    keywords = KeywordSet::DEFENDER,
    coverage = Coverage::Partial(
        "both restrictions are off the card: no static Modifier prevents \
         damage from a filtered source (`PreventDamageToIt` is combat-only and \
         unfiltered; `ProtectionFrom` also stops targeting and blocking), and \
         no `Filter` describes a spell's or ability's targeting permissions"
    ),
    // NOT SUPPORTED: "Prevent all damage that would be dealt to this creature
    // by creatures it's blocking." — `Modifier::PreventDamageToIt` prevents
    // combat damage from every source and names no source filter;
    // `Modifier::ProtectionFrom` can name creatures and does prevent their
    // damage, but it also stops them targeting and blocking this creature,
    // which the card does not say, and no filter names the creatures this Wall
    // is blocking.
    // NOT SUPPORTED: "This creature can't be the target of spells that can
    // target only Walls or of abilities that can target only Walls." —
    // `Modifier::CantBeTargetedBy` asks its filter of the targeting spell or
    // of the ability's source, but "can target only Walls" describes the
    // spell's targeting permission rather than any characteristic of the
    // source, and no `Filter` reads a targeting restriction.
    abilities = &[],
);
