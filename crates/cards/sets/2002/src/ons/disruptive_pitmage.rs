//! Disruptive Pitmage — {2}{U} — Creature — Human Wizard
//! Oracle: {T}: Counter target spell unless its controller pays {1}.
//! Oracle: Morph {U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #81 — Onslaught | Scryfall ID: 5b0d9c2f-356c-4f27-8560-8ffceadac31c | Oracle ID: c7a60f81-f7cb-4fd7-b422-963290964069
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DISRUPTIVE_PITMAGE,
    oracle_id = "c7a60f81-f7cb-4fd7-b422-963290964069",
    scryfall_id = "5b0d9c2f-356c-4f27-8560-8ffceadac31c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Disruptive Pitmage",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
