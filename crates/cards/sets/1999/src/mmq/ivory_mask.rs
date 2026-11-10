//! Ivory Mask — {2}{W}{W} — Enchantment
//! Oracle: You have shroud. (You can't be the target of spells or abilities.)
//! Set: 9ED #23 — Ninth Edition | Scryfall ID: b02422f7-728b-481b-9eb1-34d17c696ce6 | Oracle ID: 028bc726-0e86-4c53-9648-625153ae4b01
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::IVORY_MASK,
    oracle_id = "028bc726-0e86-4c53-9648-625153ae4b01",
    scryfall_id = "b02422f7-728b-481b-9eb1-34d17c696ce6",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Ivory Mask",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
