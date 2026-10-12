//! Wall of Shadows — {1}{B}{B} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: Prevent all damage that would be dealt to this creature by creatures it's blocking.
//! Oracle: This creature can't be the target of spells that can target only Walls or of abilities that can target only Walls.
//! Set: CHR #41 — Chronicles | Scryfall ID: 69c6e076-d7bf-435b-ba79-84aa9f073130 | Oracle ID: 0e6ff89e-9ae7-4dc3-98b7-05401c5df266

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
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::PreventDamageFrom(&Filter::BlockedBySource)
        ),
        static_ability!(Filter::This, Modifier::CantBeTargetedByWallOnly),
    ],
);
