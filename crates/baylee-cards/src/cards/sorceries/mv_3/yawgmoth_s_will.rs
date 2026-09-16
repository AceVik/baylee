//! Yawgmoth's Will — {2}{B} — Sorcery
//! Oracle: Until end of turn, you may play lands and cast spells from your graveyard.
//! Oracle: If a card would be put into your graveyard from anywhere this turn, exile that card instead.
//! Set: VMA #148 — Vintage Masters | Scryfall ID: 337239c7-73c4-4e2d-9160-ed26927dea1d | Oracle ID: 322f0459-f394-44f0-977b-55fd0cbe0712
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::YAWGMOTH_S_WILL,
    oracle_id = "322f0459-f394-44f0-977b-55fd0cbe0712",
    scryfall_id = "337239c7-73c4-4e2d-9160-ed26927dea1d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Yawgmoth's Will",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
