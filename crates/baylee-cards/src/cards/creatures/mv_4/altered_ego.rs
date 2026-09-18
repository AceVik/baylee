//! Altered Ego — {X}{2}{G}{U} — Creature — Shapeshifter
//! Oracle: This spell can't be countered.
//! Oracle: You may have this creature enter as a copy of any creature on the battlefield, except it enters with X additional +1/+1 counters on it.
//! Set: SOC #292 — Secrets of Strixhaven Commander | Scryfall ID: d51e076a-be33-4b2b-b52f-fe7b5bc56206 | Oracle ID: 7c35f3fd-c64e-4944-a4d5-37ce916d23c3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ALTERED_EGO,
    oracle_id = "7c35f3fd-c64e-4944-a4d5-37ce916d23c3",
    scryfall_id = "d51e076a-be33-4b2b-b52f-fe7b5bc56206",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces = &[face!(
        name = "Altered Ego",
        mana_cost = mana!("{X}{2}{G}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
