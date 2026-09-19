//! Vile Deacon — {2}{B}{B} — Creature — Human Cleric
//! Oracle: Whenever this creature attacks, it gets +X/+X until end of turn, where X is the number of Clerics on the battlefield.
//! Set: LGN #85 — Legions | Scryfall ID: b2641bd5-c845-47a1-8038-bb28b06f896e | Oracle ID: 123147b4-57d0-44cd-bdd5-a449ac86c1cb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VILE_DEACON,
    oracle_id = "123147b4-57d0-44cd-bdd5-a449ac86c1cb",
    scryfall_id = "b2641bd5-c845-47a1-8038-bb28b06f896e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Vile Deacon",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
