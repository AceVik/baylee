//! Tidal Control — {1}{U}{U} — Enchantment
//! Oracle: Cumulative upkeep {2} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: Pay 2 life or {2}: Counter target red or green spell. Any player may activate this ability.
//! Set: ALL #40 — Alliances | Scryfall ID: cb9a7b7d-3d37-4bb6-ab48-1fec2bfb4fdc | Oracle ID: 855e200e-5375-4aac-b1f7-5113162e7e14
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TIDAL_CONTROL,
    oracle_id = "855e200e-5375-4aac-b1f7-5113162e7e14",
    scryfall_id = "cb9a7b7d-3d37-4bb6-ab48-1fec2bfb4fdc",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Tidal Control",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
