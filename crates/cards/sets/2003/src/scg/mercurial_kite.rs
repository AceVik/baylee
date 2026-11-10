//! Mercurial Kite — {3}{U} — Creature — Bird
//! Oracle: Flying
//! Oracle: Whenever this creature deals combat damage to a creature, tap that creature. That creature doesn't untap during its controller's next untap step.
//! Set: SCG #39 — Scourge | Scryfall ID: a6bc8655-ae27-40be-8d61-e80a5924e955 | Oracle ID: 92f2a27d-3ec6-445c-9962-96b099f793ee
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MERCURIAL_KITE,
    oracle_id = "92f2a27d-3ec6-445c-9962-96b099f793ee",
    scryfall_id = "a6bc8655-ae27-40be-8d61-e80a5924e955",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mercurial Kite",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
