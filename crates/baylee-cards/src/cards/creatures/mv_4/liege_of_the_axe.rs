//! Liege of the Axe — {3}{W} — Creature — Human Soldier
//! Oracle: Vigilance
//! Oracle: Morph {1}{W} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, untap it.
//! Set: LGN #16 — Legions | Scryfall ID: eb518bf0-17ad-4bbf-b922-42ee76ffcbea | Oracle ID: d838969a-b219-4964-bc7e-df0348392bbe
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LIEGE_OF_THE_AXE,
    oracle_id = "d838969a-b219-4964-bc7e-df0348392bbe",
    scryfall_id = "eb518bf0-17ad-4bbf-b922-42ee76ffcbea",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Liege of the Axe",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
