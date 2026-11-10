//! Collective Restraint — {3}{U} — Enchantment
//! Oracle: Domain — Creatures can't attack you unless their controller pays {X} for each creature they control that's attacking you, where X is the number of basic land types among lands you control.
//! Set: INV #49 — Invasion | Scryfall ID: d71daa57-ac02-4dd9-8c90-d38bdd45fb51 | Oracle ID: 51f561db-ca7f-4661-bc04-99b8af02b8c9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::COLLECTIVE_RESTRAINT,
    oracle_id = "51f561db-ca7f-4661-bc04-99b8af02b8c9",
    scryfall_id = "d71daa57-ac02-4dd9-8c90-d38bdd45fb51",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Collective Restraint",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
