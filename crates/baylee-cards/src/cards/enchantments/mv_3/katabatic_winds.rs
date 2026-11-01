//! Katabatic Winds — {2}{G} — Enchantment
//! Oracle: Phasing (This phases in or out before you untap during each of your untap steps. While it's phased out, it's treated as though it doesn't exist.)
//! Oracle: Creatures with flying can't attack or block, and their activated abilities with {T} in their costs can't be activated.
//! Set: VIS #109 — Visions | Scryfall ID: 97b34ce8-1eb2-44eb-813a-09d0308e27a0 | Oracle ID: 70446c56-07de-4d7f-94ae-4da88fba41cf
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KATABATIC_WINDS,
    oracle_id = "70446c56-07de-4d7f-94ae-4da88fba41cf",
    scryfall_id = "97b34ce8-1eb2-44eb-813a-09d0308e27a0",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Katabatic Winds",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
