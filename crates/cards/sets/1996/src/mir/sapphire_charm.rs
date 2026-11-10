//! Sapphire Charm — {U} — Instant
//! Oracle: Choose one —
//! Oracle: • Target player draws a card at the beginning of the next turn's upkeep.
//! Oracle: • Target creature gains flying until end of turn.
//! Oracle: • Target creature an opponent controls phases out. (While it's phased out, it's treated as though it doesn't exist. It phases in before its controller untaps during their next untap step.)
//! Set: MIR #89 — Mirage | Scryfall ID: 2944e5df-eef9-42be-a591-5ac15e306ad8 | Oracle ID: 2ea2bf6c-a427-4d0c-b447-159959327697
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SAPPHIRE_CHARM,
    oracle_id = "2ea2bf6c-a427-4d0c-b447-159959327697",
    scryfall_id = "2944e5df-eef9-42be-a591-5ac15e306ad8",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Sapphire Charm",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
