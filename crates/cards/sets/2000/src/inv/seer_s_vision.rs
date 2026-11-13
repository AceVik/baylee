//! Seer's Vision — {2}{U}{B} — Enchantment
//! Oracle: Your opponents play with their hands revealed.
//! Oracle: Sacrifice this enchantment: Look at target player's hand and choose a card from it. That player discards that card. Activate only as a sorcery.
//! Set: INV #270 — Invasion | Scryfall ID: 0c94618a-808c-4b3c-8f34-45e64d0414d3 | Oracle ID: f5276e83-ca3f-4755-9239-e44d91a446fd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SEER_S_VISION,
    oracle_id = "f5276e83-ca3f-4755-9239-e44d91a446fd",
    scryfall_id = "0c94618a-808c-4b3c-8f34-45e64d0414d3",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(
        name = "Seer's Vision",
        mana_cost = mana!("{2}{U}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
