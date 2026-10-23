//! Melting — {3}{R} — Enchantment
//! Oracle: All lands are no longer snow.
//! Set: ICE #201 — Ice Age | Scryfall ID: 8d90065e-2c7e-44e5-9f59-015d468214bf | Oracle ID: 599247f2-7f88-40ab-be08-e284e5f231d9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MELTING,
    oracle_id = "599247f2-7f88-40ab-be08-e284e5f231d9",
    scryfall_id = "8d90065e-2c7e-44e5-9f59-015d468214bf",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Melting",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
