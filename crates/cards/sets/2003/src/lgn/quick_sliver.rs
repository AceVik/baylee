//! Quick Sliver — {1}{G} — Creature — Sliver
//! Oracle: Flash
//! Oracle: Any player may cast Sliver spells as though they had flash.
//! Set: CMM #907 — Commander Masters | Scryfall ID: f7749b2d-05a6-40ec-8d6a-8c1223cd5a09 | Oracle ID: 530a5740-041c-42df-8fe0-fcea87258038
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::QUICK_SLIVER,
    oracle_id = "530a5740-041c-42df-8fe0-fcea87258038",
    scryfall_id = "f7749b2d-05a6-40ec-8d6a-8c1223cd5a09",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Quick Sliver",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SLIVER],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
