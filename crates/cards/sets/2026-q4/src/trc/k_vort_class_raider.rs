//! K'Vort-Class Raider — {4} — Artifact — Spacecraft
//! Oracle: During your turn, this Spacecraft and creatures you control have hexproof.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 4+.)
//! Oracle: 4+ | Flying, menace, haste
//! Set: TRC #116 — Star Trek Commander | Scryfall ID: 9e2ad655-5206-4832-a350-ebfffdc180ff | Oracle ID: a52d7ab2-a750-4c3a-9f0c-999121aa81af
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::K_VORT_CLASS_RAIDER,
    oracle_id = "a52d7ab2-a750-4c3a-9f0c-999121aa81af",
    scryfall_id = "9e2ad655-5206-4832-a350-ebfffdc180ff",
    faces = &[face!(
        name = "K'Vort-Class Raider",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(5),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
