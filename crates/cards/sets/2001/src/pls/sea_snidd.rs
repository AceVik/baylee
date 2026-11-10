//! Sea Snidd — {4}{U} — Creature — Beast
//! Oracle: {T}: Target land becomes the basic land type of your choice until end of turn.
//! Set: PLS #31 — Planeshift | Scryfall ID: ca11015e-200b-488c-8bf5-662dcc03cd2d | Oracle ID: c5fdcbc9-e7da-4a88-95cf-c526f10ad96e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SEA_SNIDD,
    oracle_id = "c5fdcbc9-e7da-4a88-95cf-c526f10ad96e",
    scryfall_id = "ca11015e-200b-488c-8bf5-662dcc03cd2d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Sea Snidd",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
