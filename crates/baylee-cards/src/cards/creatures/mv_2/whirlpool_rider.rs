//! Whirlpool Rider — {1}{U} — Creature — Merfolk
//! Oracle: When this creature enters, shuffle the cards from your hand into your library, then draw that many cards.
//! Set: APC #35 — Apocalypse | Scryfall ID: 0de47f44-8c5e-4114-9064-145d2d8813c6 | Oracle ID: 13a3f2c1-0454-4661-a462-542254f8360d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WHIRLPOOL_RIDER,
    oracle_id = "13a3f2c1-0454-4661-a462-542254f8360d",
    scryfall_id = "0de47f44-8c5e-4114-9064-145d2d8813c6",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Whirlpool Rider",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
