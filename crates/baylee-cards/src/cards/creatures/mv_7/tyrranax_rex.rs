//! Tyrranax Rex — {4}{G}{G}{G} — Creature — Phyrexian Dinosaur
//! Oracle: This spell can't be countered.
//! Oracle: Trample, ward {4}, haste
//! Oracle: Toxic 4 (Players dealt combat damage by this creature also get four poison counters.)
//! Set: ONE #189 — Phyrexia: All Will Be One | Scryfall ID: 0fb52b44-da5f-4f7a-a6c2-7924b855e051 | Oracle ID: 6e42da0c-151e-468d-91cb-5a5b117a9298
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TYRRANAX_REX,
    oracle_id = "6e42da0c-151e-468d-91cb-5a5b117a9298",
    scryfall_id = "0fb52b44-da5f-4f7a-a6c2-7924b855e051",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Tyrranax Rex",
        mana_cost = mana!("{4}{G}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::DINOSAUR],
        power = Some(8),
        toughness = Some(8),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
