//! Dark Supplicant — {B} — Creature — Human Cleric
//! Oracle: {T}, Sacrifice three Clerics: Search your graveyard, hand, and/or library for a card named Scion of Darkness and put it onto the battlefield. If you search your library this way, shuffle.
//! Set: LGN #64 — Legions | Scryfall ID: eb685932-5df5-4f26-9633-b1daa8925359 | Oracle ID: dc7929cb-50a9-4662-a106-c415fc03a1a9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DARK_SUPPLICANT,
    oracle_id = "dc7929cb-50a9-4662-a106-c415fc03a1a9",
    scryfall_id = "eb685932-5df5-4f26-9633-b1daa8925359",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Dark Supplicant",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
