//! Fury — {3}{R}{R} — Creature — Elemental Incarnation
//! Oracle: Double strike
//! Oracle: When this creature enters, it deals 4 damage divided as you choose among any number of target creatures and/or planeswalkers.
//! Oracle: Evoke—Exile a red card from your hand.
//! Set: ECC #50 — Lorwyn Eclipsed Commander | Scryfall ID: 932d0fb9-382d-464a-9427-b91fa2398cdb | Oracle ID: fbf9f8c5-849f-45d5-8129-5fc683c21a04
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FURY,
    oracle_id = "fbf9f8c5-849f-45d5-8129-5fc683c21a04",
    scryfall_id = "932d0fb9-382d-464a-9427-b91fa2398cdb",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Fury",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::ELEMENTAL,
            subtypes::creature::INCARNATION
        ],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
