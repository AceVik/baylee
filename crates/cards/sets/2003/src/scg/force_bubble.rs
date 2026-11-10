//! Force Bubble — {2}{W}{W} — Enchantment
//! Oracle: If damage would be dealt to you, put that many depletion counters on this enchantment instead.
//! Oracle: When there are four or more depletion counters on this enchantment, sacrifice it.
//! Oracle: At the beginning of each end step, remove all depletion counters from this enchantment.
//! Set: SCG #14 — Scourge | Scryfall ID: 742ac116-86ed-4ce6-9805-76f47a41c4c4 | Oracle ID: 655ae8e7-372b-4d8d-b33f-4aca46831abb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FORCE_BUBBLE,
    oracle_id = "655ae8e7-372b-4d8d-b33f-4aca46831abb",
    scryfall_id = "742ac116-86ed-4ce6-9805-76f47a41c4c4",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Force Bubble",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
