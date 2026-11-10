//! Daru Mender — {W} — Creature — Human Cleric
//! Oracle: Morph {W} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, regenerate target creature.
//! Set: LGN #8 — Legions | Scryfall ID: de13fba8-3fee-4ce2-b84d-b518a99eefe0 | Oracle ID: 6072c129-9525-4630-b31a-d5d61a944d02
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DARU_MENDER,
    oracle_id = "6072c129-9525-4630-b31a-d5d61a944d02",
    scryfall_id = "de13fba8-3fee-4ce2-b84d-b518a99eefe0",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Daru Mender",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
