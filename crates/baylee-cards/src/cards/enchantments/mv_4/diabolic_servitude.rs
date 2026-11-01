//! Diabolic Servitude — {3}{B} — Enchantment
//! Oracle: When this enchantment enters, return target creature card from your graveyard to the battlefield.
//! Oracle: When the creature put onto the battlefield with this enchantment dies, exile it and return this enchantment to its owner's hand.
//! Oracle: When this enchantment leaves the battlefield, exile the creature put onto the battlefield with this enchantment.
//! Set: USG #130 — Urza's Saga | Scryfall ID: 0cb1e0c9-9041-44cd-9e96-87cf14c67068 | Oracle ID: e275cbfb-3d25-4189-84f0-3162eadc7d65
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DIABOLIC_SERVITUDE,
    oracle_id = "e275cbfb-3d25-4189-84f0-3162eadc7d65",
    scryfall_id = "0cb1e0c9-9041-44cd-9e96-87cf14c67068",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Diabolic Servitude",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
