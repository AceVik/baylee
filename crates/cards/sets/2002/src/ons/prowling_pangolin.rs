//! Prowling Pangolin — {3}{B}{B} — Creature — Pangolin Beast
//! Oracle: When this creature enters, any player may sacrifice two creatures of their choice. If a player does, sacrifice this creature.
//! Set: EMA #104 — Eternal Masters | Scryfall ID: b6bf8191-3154-48d7-a49b-4d07b5e35a15 | Oracle ID: 92042cad-09f5-464f-85ad-baad27e6d1eb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PROWLING_PANGOLIN,
    oracle_id = "92042cad-09f5-464f-85ad-baad27e6d1eb",
    scryfall_id = "b6bf8191-3154-48d7-a49b-4d07b5e35a15",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Prowling Pangolin",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PANGOLIN, subtypes::creature::BEAST],
        power = Some(6),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
