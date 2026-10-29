//! Elvish Guidance — {2}{G} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: Whenever enchanted land is tapped for mana, its controller adds an additional {G} for each Elf on the battlefield.
//! Set: ONS #255 — Onslaught | Scryfall ID: 8698c46b-2628-4482-88f9-e37a01ade274 | Oracle ID: 8d6b94d5-bb53-4941-8c6b-7ebef764c1ce
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ELVISH_GUIDANCE,
    oracle_id = "8d6b94d5-bb53-4941-8c6b-7ebef764c1ce",
    scryfall_id = "8698c46b-2628-4482-88f9-e37a01ade274",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Elvish Guidance",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
