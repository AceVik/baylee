//! Show and Tell — {2}{U} — Sorcery
//! Oracle: Each player may put an artifact, creature, enchantment, or land card from their hand onto the battlefield.
//! Set: CN2 #121 — Conspiracy: Take the Crown | Scryfall ID: fa7b7897-36e0-415a-8bb7-602886164852 | Oracle ID: b83a3ba0-249e-4c39-bbf0-cb005413f7d2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHOW_AND_TELL,
    oracle_id = "b83a3ba0-249e-4c39-bbf0-cb005413f7d2",
    scryfall_id = "fa7b7897-36e0-415a-8bb7-602886164852",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Show and Tell",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
