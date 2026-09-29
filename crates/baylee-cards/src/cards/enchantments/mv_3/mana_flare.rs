//! Mana Flare — {2}{R} — Enchantment
//! Oracle: Whenever a player taps a land for mana, that player adds one mana of any type that land produced.
//! Set: ME1 #103 — Masters Edition | Scryfall ID: a4bb303d-b230-430a-a2c3-f91a776de34e | Oracle ID: 97159138-c34b-416e-b079-5c952383a243
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MANA_FLARE,
    oracle_id = "97159138-c34b-416e-b079-5c952383a243",
    scryfall_id = "a4bb303d-b230-430a-a2c3-f91a776de34e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Mana Flare",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
