//! Retreat to Kazandu — {2}{G} — Enchantment
//! Oracle: Landfall — Whenever a land you control enters, choose one —
//! Oracle: • Put a +1/+1 counter on target creature.
//! Oracle: • You gain 2 life.
//! Set: CMR #435 — Commander Legends | Scryfall ID: 553ac818-f476-43fd-841c-c91d78c2506e | Oracle ID: 3f8e5ff1-af89-427e-924c-19a44f9a3788
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RETREAT_TO_KAZANDU,
    oracle_id = "3f8e5ff1-af89-427e-924c-19a44f9a3788",
    scryfall_id = "553ac818-f476-43fd-841c-c91d78c2506e",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Retreat to Kazandu",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
