//! Sealed Fate — {X}{U}{B} — Sorcery
//! Oracle: Look at the top X cards of target opponent's library. Exile one of those cards and put the rest back on top of that player's library in any order.
//! Set: MIR #282 — Mirage | Scryfall ID: 36cf9da8-f078-4f6e-8077-bb24a7ed487f | Oracle ID: 0b9b5856-9396-4a55-8675-58cd171df708
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SEALED_FATE,
    oracle_id = "0b9b5856-9396-4a55-8675-58cd171df708",
    scryfall_id = "36cf9da8-f078-4f6e-8077-bb24a7ed487f",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(
        name = "Sealed Fate",
        mana_cost = mana!("{X}{U}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
