//! Whirlpool Warrior — {2}{U} — Creature — Merfolk Warrior
//! Oracle: When this creature enters, shuffle the cards from your hand into your library, then draw that many cards.
//! Oracle: {R}, Sacrifice this creature: Each player shuffles the cards from their hand into their library, then draws that many cards.
//! Set: PCA #29 — Planechase Anthology | Scryfall ID: 10587371-5f50-4caa-ab2e-7d57b721b5e8 | Oracle ID: b7a1c54b-bb2f-4423-865a-ad114cd18c76
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WHIRLPOOL_WARRIOR,
    oracle_id = "b7a1c54b-bb2f-4423-865a-ad114cd18c76",
    scryfall_id = "10587371-5f50-4caa-ab2e-7d57b721b5e8",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces = &[face!(
        name = "Whirlpool Warrior",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK, subtypes::creature::WARRIOR],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
