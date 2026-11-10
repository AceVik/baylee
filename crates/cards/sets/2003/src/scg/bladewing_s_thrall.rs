//! Bladewing's Thrall — {2}{B}{B} — Creature — Zombie
//! Oracle: This creature has flying as long as you control a Dragon.
//! Oracle: When a Dragon enters, you may return this card from your graveyard to the battlefield.
//! Set: IMA #81 — Iconic Masters | Scryfall ID: 9d63d3d8-5699-4577-a908-4415bc96b404 | Oracle ID: f068a0f3-48bc-4d5d-be79-c23c0be11c27
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BLADEWING_S_THRALL,
    oracle_id = "f068a0f3-48bc-4d5d-be79-c23c0be11c27",
    scryfall_id = "9d63d3d8-5699-4577-a908-4415bc96b404",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Bladewing's Thrall",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
