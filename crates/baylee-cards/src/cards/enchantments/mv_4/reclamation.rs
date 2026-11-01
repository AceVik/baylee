//! Reclamation — {2}{G}{W} — Enchantment
//! Oracle: Black creatures can't attack unless their controller sacrifices a land of their choice for each black creature they control that's attacking. (This cost is paid as attackers are declared.)
//! Set: ICE #300 — Ice Age | Scryfall ID: ca335f4f-d345-4eb9-9bc6-74595c501078 | Oracle ID: ff942899-a70d-4fca-b125-9712aa635fff
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RECLAMATION,
    oracle_id = "ff942899-a70d-4fca-b125-9712aa635fff",
    scryfall_id = "ca335f4f-d345-4eb9-9bc6-74595c501078",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(
        name = "Reclamation",
        mana_cost = mana!("{2}{G}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
