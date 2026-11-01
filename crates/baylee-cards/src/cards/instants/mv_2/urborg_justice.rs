//! Urborg Justice — {B}{B} — Instant
//! Oracle: Target opponent sacrifices a creature of their choice for each creature put into your graveyard from the battlefield this turn.
//! Set: WTH #84 — Weatherlight | Scryfall ID: 39f322ff-0b04-41ce-90cd-9896f941e703 | Oracle ID: d8e2dea9-0e96-4ce4-a070-9893f44cf0cc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::URBORG_JUSTICE,
    oracle_id = "d8e2dea9-0e96-4ce4-a070-9893f44cf0cc",
    scryfall_id = "39f322ff-0b04-41ce-90cd-9896f941e703",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Urborg Justice",
        mana_cost = mana!("{B}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
