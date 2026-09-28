//! Subtlety — {2}{U}{U} — Creature — Elemental Incarnation
//! Oracle: Flash
//! Oracle: Flying
//! Oracle: When this creature enters, choose up to one target creature spell or planeswalker spell. Its owner puts it on their choice of the top or bottom of their library.
//! Oracle: Evoke—Exile a blue card from your hand.
//! Set: MH2 #67 — Modern Horizons 2 | Scryfall ID: 701256d5-1389-48b7-9581-d6037209bd06 | Oracle ID: 377179d5-ac83-4d34-b5b1-f3d8caa60f79
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SUBTLETY,
    oracle_id = "377179d5-ac83-4d34-b5b1-f3d8caa60f79",
    scryfall_id = "701256d5-1389-48b7-9581-d6037209bd06",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Subtlety",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::ELEMENTAL,
            subtypes::creature::INCARNATION
        ],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
