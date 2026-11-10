//! Withdraw — {U}{U} — Instant
//! Oracle: Return target creature to its owner's hand. Then return another target creature to its owner's hand unless its controller pays {1}.
//! Set: PCY #54 — Prophecy | Scryfall ID: f1a3a52f-0ccd-4935-b3ca-9c69cba283cc | Oracle ID: 626882c5-15a8-43ea-ab08-33a7e949c88c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WITHDRAW,
    oracle_id = "626882c5-15a8-43ea-ab08-33a7e949c88c",
    scryfall_id = "f1a3a52f-0ccd-4935-b3ca-9c69cba283cc",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Withdraw",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
