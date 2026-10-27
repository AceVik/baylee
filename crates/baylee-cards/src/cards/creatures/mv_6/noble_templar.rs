//! Noble Templar — {5}{W} — Creature — Human Cleric Soldier
//! Oracle: Vigilance
//! Oracle: Plainscycling {2} ({2}, Discard this card: Search your library for a Plains card, reveal it, put it into your hand, then shuffle.)
//! Set: A25 #25 — Masters 25 | Scryfall ID: 82073551-8106-4e88-8d55-913d86bbe48a | Oracle ID: e7079969-8d88-4f18-a872-2661bed6ff74
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NOBLE_TEMPLAR,
    oracle_id = "e7079969-8d88-4f18-a872-2661bed6ff74",
    scryfall_id = "82073551-8106-4e88-8d55-913d86bbe48a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Noble Templar",
        mana_cost = mana!("{5}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::CLERIC,
            subtypes::creature::SOLDIER
        ],
        power = Some(3),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
