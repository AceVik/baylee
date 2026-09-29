//! Wild Growth — {G} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: Whenever enchanted land is tapped for mana, its controller adds an additional {G}.
//! Set: MKC #195 — Murders at Karlov Manor Commander | Scryfall ID: 47260e7c-29bf-46f1-a029-9da7bbb418b5 | Oracle ID: 706ae742-1807-44b7-a4fa-f2e26f61519a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WILD_GROWTH,
    oracle_id = "706ae742-1807-44b7-a4fa-f2e26f61519a",
    scryfall_id = "47260e7c-29bf-46f1-a029-9da7bbb418b5",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Wild Growth",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
