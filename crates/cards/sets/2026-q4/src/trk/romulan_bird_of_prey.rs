//! Romulan Bird-of-Prey — {1}{U} — Artifact — Spacecraft
//! Oracle: Cloaking Device — As long as this Spacecraft isn't a creature, it has hexproof.
//! Oracle: Whenever you cast a noncreature spell, if this Spacecraft isn't a creature, put two charge counters on it.
//! Oracle: Station
//! Oracle: 8+ | Flying
//! Set: TRK #77 — Star Trek | Scryfall ID: 6a461d90-6b7b-4777-958a-8a5d8365ad8c | Oracle ID: e614f97b-da8e-4569-9bdd-f76371c66836
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ROMULAN_BIRD_OF_PREY,
    oracle_id = "e614f97b-da8e-4569-9bdd-f76371c66836",
    scryfall_id = "6a461d90-6b7b-4777-958a-8a5d8365ad8c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Romulan Bird-of-Prey",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
