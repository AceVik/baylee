//! Hystrodon — {4}{G} — Creature — Beast
//! Oracle: Trample
//! Oracle: Whenever this creature deals combat damage to a player, you may draw a card.
//! Oracle: Morph {1}{G}{G} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #266 — Onslaught | Scryfall ID: 1c964473-7c54-4c2d-a3eb-dba01c842103 | Oracle ID: 17191fa9-e956-4f46-b541-465790125b6b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HYSTRODON,
    oracle_id = "17191fa9-e956-4f46-b541-465790125b6b",
    scryfall_id = "1c964473-7c54-4c2d-a3eb-dba01c842103",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Hystrodon",
        mana_cost = mana!("{4}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
