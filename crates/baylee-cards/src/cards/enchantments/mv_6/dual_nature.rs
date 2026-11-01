//! Dual Nature — {4}{G}{G} — Enchantment
//! Oracle: Whenever a nontoken creature enters, its controller creates a token that's a copy of that creature.
//! Oracle: Whenever a nontoken creature leaves the battlefield, exile all tokens with the same name as that creature.
//! Oracle: When this enchantment leaves the battlefield, exile all tokens created with this enchantment.
//! Set: PCY #112 — Prophecy | Scryfall ID: 6890e414-d7e1-4320-924c-083e65a2ae72 | Oracle ID: 7a8fcf6f-3bda-485c-b960-62a37112f0eb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DUAL_NATURE,
    oracle_id = "7a8fcf6f-3bda-485c-b960-62a37112f0eb",
    scryfall_id = "6890e414-d7e1-4320-924c-083e65a2ae72",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Dual Nature",
        mana_cost = mana!("{4}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
