//! Rising Waters — {3}{U} — Enchantment
//! Oracle: Lands don't untap during their controllers' untap steps.
//! Oracle: At the beginning of each player's upkeep, that player untaps a land they control.
//! Set: NEM #38 — Nemesis | Scryfall ID: ec9c84db-cf45-43e1-b38f-8bbf53cf088b | Oracle ID: d4cbf380-1a05-48ea-9187-de931b00866d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RISING_WATERS,
    oracle_id = "d4cbf380-1a05-48ea-9187-de931b00866d",
    scryfall_id = "ec9c84db-cf45-43e1-b38f-8bbf53cf088b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Rising Waters",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
