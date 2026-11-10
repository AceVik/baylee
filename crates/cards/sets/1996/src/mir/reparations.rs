//! Reparations — {1}{W}{U} — Enchantment
//! Oracle: Whenever an opponent casts a spell that targets you or a creature you control, you may draw a card.
//! Set: MIR #278 — Mirage | Scryfall ID: 0a9edf28-79c0-42a5-af0f-6df9c3a1f546 | Oracle ID: 3cb86deb-f4e5-41c5-bfcd-614a24f3499c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REPARATIONS,
    oracle_id = "3cb86deb-f4e5-41c5-bfcd-614a24f3499c",
    scryfall_id = "0a9edf28-79c0-42a5-af0f-6df9c3a1f546",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[face!(
        name = "Reparations",
        mana_cost = mana!("{1}{W}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
