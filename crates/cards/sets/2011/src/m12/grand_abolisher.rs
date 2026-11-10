//! Grand Abolisher — {W}{W} — Creature — Human Cleric
//! Oracle: During your turn, your opponents can't cast spells or activate abilities of artifacts, creatures, or enchantments.
//! Set: BIG #2 — The Big Score | Scryfall ID: ee793ed2-7d59-4640-8868-ad486600df2c | Oracle ID: c749f23c-40c0-4159-b84c-a70cbb062c14
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GRAND_ABOLISHER,
    oracle_id = "c749f23c-40c0-4159-b84c-a70cbb062c14",
    scryfall_id = "ee793ed2-7d59-4640-8868-ad486600df2c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Grand Abolisher",
        mana_cost = mana!("{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
