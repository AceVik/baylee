//! Torrent of Fire — {3}{R}{R} — Sorcery
//! Oracle: Torrent of Fire deals damage to any target equal to the greatest mana value among permanents you control.
//! Set: SCG #107 — Scourge | Scryfall ID: feeee859-f64a-4cd8-be0b-ad60cff8812e | Oracle ID: 00181784-9213-492a-ba8c-2028969b049e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TORRENT_OF_FIRE,
    oracle_id = "00181784-9213-492a-ba8c-2028969b049e",
    scryfall_id = "feeee859-f64a-4cd8-be0b-ad60cff8812e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Torrent of Fire",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
