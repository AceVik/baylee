//! Tariff — {1}{W} — Sorcery
//! Oracle: Each player sacrifices the creature they control with the greatest mana value unless they pay that creature's mana cost. If two or more creatures a player controls are tied for greatest, that player chooses one.
//! Set: 6ED #47 — Classic Sixth Edition | Scryfall ID: 61094858-b8a8-425f-9c96-fac4fc6ecae8 | Oracle ID: e96b0560-1d91-48df-b42f-3b7d9c6d94bd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TARIFF,
    oracle_id = "e96b0560-1d91-48df-b42f-3b7d9c6d94bd",
    scryfall_id = "61094858-b8a8-425f-9c96-fac4fc6ecae8",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Tariff",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
