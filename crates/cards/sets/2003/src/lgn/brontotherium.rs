//! Brontotherium — {4}{G}{G} — Creature — Beast
//! Oracle: Trample
//! Oracle: Provoke (Whenever this creature attacks, you may have target creature defending player controls untap and block it if able.)
//! Set: LGN #119 — Legions | Scryfall ID: a171f5e2-ed3d-4675-a4fc-953ebb907aa0 | Oracle ID: 6e3301a5-a99b-4b17-aa40-34ccc5904977
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BRONTOTHERIUM,
    oracle_id = "6e3301a5-a99b-4b17-aa40-34ccc5904977",
    scryfall_id = "a171f5e2-ed3d-4675-a4fc-953ebb907aa0",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Brontotherium",
        mana_cost = mana!("{4}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(5),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
