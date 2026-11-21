//! Neonate's Rush — {2}{R} — Instant
//! Oracle: This spell costs {1} less to cast if you control a Vampire.
//! Oracle: Neonate's Rush deals 1 damage to target creature and 1 damage to its controller. Draw a card.
//! Set: INR #166 — Innistrad Remastered | Scryfall ID: 1d623e71-05bd-46d6-b831-2c8dcff7ab45 | Oracle ID: 721aeedc-1de5-40d0-a04c-a2ebc056d06a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NEONATE_S_RUSH,
    oracle_id = "721aeedc-1de5-40d0-a04c-a2ebc056d06a",
    scryfall_id = "1d623e71-05bd-46d6-b831-2c8dcff7ab45",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Neonate's Rush",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
