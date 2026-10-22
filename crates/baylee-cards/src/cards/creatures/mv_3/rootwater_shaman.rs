//! Rootwater Shaman — {2}{U} — Creature — Merfolk Shaman
//! Oracle: You may cast Aura spells with enchant creature as though they had flash.
//! Set: TMP #84 — Tempest | Scryfall ID: caa1b84b-efda-4324-9106-0d1d00385cdc | Oracle ID: 8a449915-8bd7-4a9c-abb1-a95852f33695
// PARTIAL: lets you cast enchantment spells as though they had flash; restricting the flash permission to Aura spells with enchant creature is not in Modifier::SpellsHaveFlash.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ROOTWATER_SHAMAN,
    oracle_id = "8a449915-8bd7-4a9c-abb1-a95852f33695",
    scryfall_id = "caa1b84b-efda-4324-9106-0d1d00385cdc",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "lets you cast enchantment spells as though they had flash; restricting the flash permission to Aura spells with enchant creature is not in Modifier::SpellsHaveFlash",
    ),
    faces = &[face!(
        name = "Rootwater Shaman",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK, subtypes::creature::SHAMAN],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::SpellsHaveFlash(Some(TypeSet::ENCHANTMENT)),
    )],
);
