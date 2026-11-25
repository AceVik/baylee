//! Extinguisher Battleship — {8} — Artifact — Spacecraft
//! Oracle: When this Spacecraft enters, destroy target noncreature permanent. Then this Spacecraft deals 4 damage to each creature.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 5+.)
//! Oracle: 5+ | Flying, trample
//! Set: EOE #242 — Edge of Eternities | Scryfall ID: 5541cdd2-84a6-4667-83eb-fffbe5b3cd3d | Oracle ID: cce3dcc3-57bb-4b95-8b70-337c67bb3c4e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EXTINGUISHER_BATTLESHIP,
    oracle_id = "cce3dcc3-57bb-4b95-8b70-337c67bb3c4e",
    scryfall_id = "5541cdd2-84a6-4667-83eb-fffbe5b3cd3d",
    faces = &[face!(
        name = "Extinguisher Battleship",
        mana_cost = mana!("{8}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(10),
        toughness = Some(10),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
