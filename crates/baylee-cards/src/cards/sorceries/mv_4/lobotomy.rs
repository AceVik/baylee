//! Lobotomy — {2}{U}{B} — Sorcery
//! Oracle: Target player reveals their hand, then you choose a card other than a basic land card from it. Search that player's graveyard, hand, and library for all cards with the same name as the chosen card and exile them. Then that player shuffles.
//! Set: INV #255 — Invasion | Scryfall ID: ff307dbb-4ab6-457b-be56-47106864bf61 | Oracle ID: a7ebe748-c3de-402f-8a6e-236a6413c0f8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LOBOTOMY,
    oracle_id = "a7ebe748-c3de-402f-8a6e-236a6413c0f8",
    scryfall_id = "ff307dbb-4ab6-457b-be56-47106864bf61",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(
        name = "Lobotomy",
        mana_cost = mana!("{2}{U}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
