//! Defender of the Order — {3}{W} — Creature — Human Cleric
//! Oracle: Morph {W}{W} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, creatures you control get +0/+2 until end of turn.
//! Set: LGN #11 — Legions | Scryfall ID: 236b1c88-20a0-479e-91fb-16bb77f699fe | Oracle ID: f9f4c574-ddbe-4ebc-bfda-29cf3e4b7c44
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DEFENDER_OF_THE_ORDER,
    oracle_id = "f9f4c574-ddbe-4ebc-bfda-29cf3e4b7c44",
    scryfall_id = "236b1c88-20a0-479e-91fb-16bb77f699fe",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Defender of the Order",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
