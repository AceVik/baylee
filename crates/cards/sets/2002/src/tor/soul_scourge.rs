//! Soul Scourge — {4}{B} — Creature — Nightmare Horror
//! Oracle: Flying
//! Oracle: When this creature enters, target player loses 3 life.
//! Oracle: When this creature leaves the battlefield, that player gains 3 life.
//! Set: TOR #85 — Torment | Scryfall ID: 4aa7d91b-cfbb-4da0-97fa-3c0a15f1dbb9 | Oracle ID: d66e89a3-d582-4867-899c-f560b8a5b973
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SOUL_SCOURGE,
    oracle_id = "d66e89a3-d582-4867-899c-f560b8a5b973",
    scryfall_id = "4aa7d91b-cfbb-4da0-97fa-3c0a15f1dbb9",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Soul Scourge",
        mana_cost = mana!("{4}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::NIGHTMARE, subtypes::creature::HORROR],
        power = Some(3),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
