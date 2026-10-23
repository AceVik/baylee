//! Martyrdom — {1}{W}{W} — Instant
//! Oracle: Until end of turn, target creature you control gains "{0}: The next 1 damage that would be dealt to target creature, planeswalker, or player this turn is dealt to this creature instead." Only you may activate this ability.
//! Set: ALL #10a — Alliances | Scryfall ID: 6ab5775a-7138-4526-83fe-350127695224 | Oracle ID: f6a6da20-52c8-4921-9884-29d3a3051b0d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MARTYRDOM,
    oracle_id = "f6a6da20-52c8-4921-9884-29d3a3051b0d",
    scryfall_id = "6ab5775a-7138-4526-83fe-350127695224",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Martyrdom",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
