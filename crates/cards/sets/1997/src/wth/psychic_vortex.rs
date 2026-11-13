//! Psychic Vortex — {2}{U}{U} — Enchantment
//! Oracle: Cumulative upkeep—Draw a card. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: At the beginning of your end step, sacrifice a land and discard your hand.
//! Set: WTH #50 — Weatherlight | Scryfall ID: 3bc2a419-7122-4eeb-bb64-738a647cfd82 | Oracle ID: 451e35dd-e0b5-4157-8511-bacb315e19bd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PSYCHIC_VORTEX,
    oracle_id = "451e35dd-e0b5-4157-8511-bacb315e19bd",
    scryfall_id = "3bc2a419-7122-4eeb-bb64-738a647cfd82",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Psychic Vortex",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
