//! Song of Serenity — {1}{G} — Enchantment
//! Oracle: Creatures that are enchanted can't attack or block.
//! Set: EXO #125 — Exodus | Scryfall ID: 2ba85b2f-37da-4595-9880-8e9f1ddbac09 | Oracle ID: 4c543799-1b72-4f32-a5be-475c0134ac0f
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;

static ENCHANTED_CREATURE: Filter =
    Filter::And(&[Filter::HasType(TypeSet::CREATURE), Filter::Enchanted]);

card!(
    index = index::SONG_OF_SERENITY,
    oracle_id = "4c543799-1b72-4f32-a5be-475c0134ac0f",
    scryfall_id = "2ba85b2f-37da-4595-9880-8e9f1ddbac09",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Song of Serenity",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        static_ability!(ENCHANTED_CREATURE, Modifier::CantAttack),
        static_ability!(ENCHANTED_CREATURE, Modifier::CantBlock(&Filter::CREATURE)),
    ],
);
