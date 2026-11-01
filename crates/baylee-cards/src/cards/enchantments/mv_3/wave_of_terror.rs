//! Wave of Terror — {2}{B} — Enchantment
//! Oracle: Cumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: At the beginning of your draw step, destroy each creature with mana value equal to the number of age counters on this enchantment. They can't be regenerated.
//! Set: WTH #86 — Weatherlight | Scryfall ID: d40ab3e7-9abb-4acc-9932-de03b533722f | Oracle ID: 2346be19-7344-4f11-be2c-a22750cb7e1d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WAVE_OF_TERROR,
    oracle_id = "2346be19-7344-4f11-be2c-a22750cb7e1d",
    scryfall_id = "d40ab3e7-9abb-4acc-9932-de03b533722f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Wave of Terror",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
