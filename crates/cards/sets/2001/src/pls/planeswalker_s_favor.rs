//! Planeswalker's Favor — {2}{G} — Enchantment
//! Oracle: {3}{G}: Target opponent reveals a card at random from their hand. Target creature gets +X/+X until end of turn, where X is the revealed card's mana value.
//! Set: PLS #86 — Planeshift | Scryfall ID: b3387540-93bf-451e-8e7a-fc78caab42b0 | Oracle ID: 1990753c-06a7-4e1f-bc0e-905a9ba6e293
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PLANESWALKER_S_FAVOR,
    oracle_id = "1990753c-06a7-4e1f-bc0e-905a9ba6e293",
    scryfall_id = "b3387540-93bf-451e-8e7a-fc78caab42b0",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Planeswalker's Favor",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
