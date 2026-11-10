//! Grip of Chaos — {4}{R}{R} — Enchantment
//! Oracle: Whenever a spell or ability is put onto the stack, if it has a single target, reselect its target at random. (Select from among all legal targets.)
//! Set: SCG #98 — Scourge | Scryfall ID: defbbd3a-0e7d-4af2-b25f-9003ddad0bf5 | Oracle ID: ca2e0c8f-7ece-465f-9103-ba60712eacf7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GRIP_OF_CHAOS,
    oracle_id = "ca2e0c8f-7ece-465f-9103-ba60712eacf7",
    scryfall_id = "defbbd3a-0e7d-4af2-b25f-9003ddad0bf5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Grip of Chaos",
        mana_cost = mana!("{4}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
