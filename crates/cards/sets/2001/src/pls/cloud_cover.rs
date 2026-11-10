//! Cloud Cover — {2}{W}{U} — Enchantment
//! Oracle: Whenever another permanent you control becomes the target of a spell or ability an opponent controls, you may return that permanent to its owner's hand.
//! Set: PLS #98 — Planeshift | Scryfall ID: 943b3886-5556-474f-8dc1-18219e25abc3 | Oracle ID: a4cd8dd1-adb9-4c4c-bda9-6f6b6c167702
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CLOUD_COVER,
    oracle_id = "a4cd8dd1-adb9-4c4c-bda9-6f6b6c167702",
    scryfall_id = "943b3886-5556-474f-8dc1-18219e25abc3",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[face!(
        name = "Cloud Cover",
        mana_cost = mana!("{2}{W}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
