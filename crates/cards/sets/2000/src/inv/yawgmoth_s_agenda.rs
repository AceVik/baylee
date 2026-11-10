//! Yawgmoth's Agenda — {3}{B}{B} — Enchantment
//! Oracle: You can't cast more than one spell each turn.
//! Oracle: You may play lands and cast spells from your graveyard.
//! Oracle: If a card would be put into your graveyard from anywhere, exile it instead.
//! Set: INV #135 — Invasion | Scryfall ID: 50f7ea7f-4f17-4f78-b68e-693e265ca829 | Oracle ID: 72177c93-ed1f-47bd-99a6-ad229a292d46
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::YAWGMOTH_S_AGENDA,
    oracle_id = "72177c93-ed1f-47bd-99a6-ad229a292d46",
    scryfall_id = "50f7ea7f-4f17-4f78-b68e-693e265ca829",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Yawgmoth's Agenda",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
