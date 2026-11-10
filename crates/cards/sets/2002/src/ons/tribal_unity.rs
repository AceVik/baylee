//! Tribal Unity — {X}{2}{G} — Instant
//! Oracle: Creatures of the creature type of your choice get +X/+X until end of turn.
//! Set: HOP #80 — Planechase | Scryfall ID: f1379753-0a37-4e33-ac46-8ef33cb98b06 | Oracle ID: 6b4ee935-4474-4757-9fab-978a2e1e72a0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TRIBAL_UNITY,
    oracle_id = "6b4ee935-4474-4757-9fab-978a2e1e72a0",
    scryfall_id = "f1379753-0a37-4e33-ac46-8ef33cb98b06",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Tribal Unity",
        mana_cost = mana!("{X}{2}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
