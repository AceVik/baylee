//! Swift Reconfiguration — {W} — Enchantment — Aura
//! Oracle: Flash
//! Oracle: Enchant creature or Vehicle
//! Oracle: Enchanted permanent is a Vehicle artifact with crew 5 and it loses all other card types. (It's not a creature unless it's crewed.)
//! Set: NEC #10 — Neon Dynasty Commander | Scryfall ID: 975dcfab-0281-4fee-92aa-021ea6c524c7 | Oracle ID: 5d47e820-913f-441a-a6cc-37ab3181d79a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SWIFT_RECONFIGURATION,
    oracle_id = "5d47e820-913f-441a-a6cc-37ab3181d79a",
    scryfall_id = "975dcfab-0281-4fee-92aa-021ea6c524c7",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Swift Reconfiguration",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
