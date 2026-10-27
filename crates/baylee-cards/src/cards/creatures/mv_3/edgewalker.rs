//! Edgewalker — {1}{W}{B} — Creature — Human Cleric
//! Oracle: Cleric spells you cast cost {W}{B} less to cast. This effect reduces only the amount of colored mana you pay. (For example, if you cast a Cleric spell with mana cost {1}{W}, it costs {1} to cast.)
//! Set: SCG #137 — Scourge | Scryfall ID: c8b477c2-2cd5-41f2-8754-d4d5000df58d | Oracle ID: 71a7f695-06b9-461d-9646-ffb975b375fc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EDGEWALKER,
    oracle_id = "71a7f695-06b9-461d-9646-ffb975b375fc",
    scryfall_id = "c8b477c2-2cd5-41f2-8754-d4d5000df58d",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::White]),
    faces = &[face!(
        name = "Edgewalker",
        mana_cost = mana!("{1}{W}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
