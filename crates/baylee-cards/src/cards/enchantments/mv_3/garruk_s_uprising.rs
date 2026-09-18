//! Garruk's Uprising — {2}{G} — Enchantment
//! Oracle: When this enchantment enters, if you control a creature with power 4 or greater, draw a card.
//! Oracle: Creatures you control have trample. (Each of those creatures can deal excess combat damage to the player or planeswalker it's attacking.)
//! Oracle: Whenever a creature you control with power 4 or greater enters, draw a card.
//! Set: ECC #109 — Lorwyn Eclipsed Commander | Scryfall ID: b58c4033-f764-42f6-966f-b7202a2babbf | Oracle ID: 3127ae9b-a7a7-43ec-89d7-688f8445b33d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GARRUK_S_UPRISING,
    oracle_id = "3127ae9b-a7a7-43ec-89d7-688f8445b33d",
    scryfall_id = "b58c4033-f764-42f6-966f-b7202a2babbf",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Garruk's Uprising",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
