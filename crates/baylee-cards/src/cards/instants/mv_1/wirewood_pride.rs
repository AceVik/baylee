//! Wirewood Pride — {G} — Instant
//! Oracle: Target creature gets +X/+X until end of turn, where X is the number of Elves on the battlefield.
//! Set: ONS #303 — Onslaught | Scryfall ID: a559e844-06c9-4953-bc2c-a58e4170fe47 | Oracle ID: ff19f10c-777c-4688-b1ab-99e53afaf629
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WIREWOOD_PRIDE,
    oracle_id = "ff19f10c-777c-4688-b1ab-99e53afaf629",
    scryfall_id = "a559e844-06c9-4953-bc2c-a58e4170fe47",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Wirewood Pride",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
