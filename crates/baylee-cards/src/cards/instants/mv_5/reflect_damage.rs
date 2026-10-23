//! Reflect Damage — {3}{R}{W} — Instant
//! Oracle: The next time a source of your choice would deal damage this turn, that damage is dealt to that source's controller instead.
//! Set: MIR #277 — Mirage | Scryfall ID: 3a2bf39b-9665-426b-b618-eb731d24a1ee | Oracle ID: 21565b0d-f814-49ca-8613-f40040c4ba6c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REFLECT_DAMAGE,
    oracle_id = "21565b0d-f814-49ca-8613-f40040c4ba6c",
    scryfall_id = "3a2bf39b-9665-426b-b618-eb731d24a1ee",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    faces = &[face!(
        name = "Reflect Damage",
        mana_cost = mana!("{3}{R}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
