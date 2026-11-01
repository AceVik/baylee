//! Planeswalker's Scorn — {2}{B} — Enchantment
//! Oracle: {3}{B}: Target opponent reveals a card at random from their hand. Target creature gets -X/-X until end of turn, where X is the revealed card's mana value. Activate only as a sorcery.
//! Set: PLS #52 — Planeshift | Scryfall ID: 8ed08376-836f-4313-83d0-481895ead9da | Oracle ID: 52cebfb0-0302-40d9-9a3f-f875dbf43659
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PLANESWALKER_S_SCORN,
    oracle_id = "52cebfb0-0302-40d9-9a3f-f875dbf43659",
    scryfall_id = "8ed08376-836f-4313-83d0-481895ead9da",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Planeswalker's Scorn",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
