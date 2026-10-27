//! Frontline Strategist — {W} — Creature — Human Soldier
//! Oracle: Morph {W} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, prevent all combat damage non-Soldier creatures would deal this turn.
//! Set: SCG #15 — Scourge | Scryfall ID: 1c43fac2-62fb-4924-848d-a8d739773d6e | Oracle ID: 6e202114-48f3-46d2-bc54-df766d149d9d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FRONTLINE_STRATEGIST,
    oracle_id = "6e202114-48f3-46d2-bc54-df766d149d9d",
    scryfall_id = "1c43fac2-62fb-4924-848d-a8d739773d6e",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Frontline Strategist",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
