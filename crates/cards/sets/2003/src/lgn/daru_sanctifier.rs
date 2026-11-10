//! Daru Sanctifier — {3}{W} — Creature — Human Cleric
//! Oracle: Morph {1}{W} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, destroy target enchantment.
//! Set: LGN #9 — Legions | Scryfall ID: 38b14a24-c74a-4465-9b36-8f5309e0a333 | Oracle ID: 7e81539e-b790-41eb-bd5e-d1de130b46a1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DARU_SANCTIFIER,
    oracle_id = "7e81539e-b790-41eb-bd5e-d1de130b46a1",
    scryfall_id = "38b14a24-c74a-4465-9b36-8f5309e0a333",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Daru Sanctifier",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
