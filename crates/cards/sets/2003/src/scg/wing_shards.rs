//! Wing Shards — {1}{W}{W} — Instant
//! Oracle: Target player sacrifices an attacking creature of their choice.
//! Oracle: Storm (When you cast this spell, copy it for each spell cast before it this turn. You may choose new targets for the copies.)
//! Set: MH1 #38 — Modern Horizons | Scryfall ID: f8326bd2-83a8-4600-b12a-0bda47168f7b | Oracle ID: b302bce5-d7ad-46d8-9dbf-8159e8709b4f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WING_SHARDS,
    oracle_id = "b302bce5-d7ad-46d8-9dbf-8159e8709b4f",
    scryfall_id = "f8326bd2-83a8-4600-b12a-0bda47168f7b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Wing Shards",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
