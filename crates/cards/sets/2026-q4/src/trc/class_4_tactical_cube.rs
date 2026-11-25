//! Class-4 Tactical Cube — {4} — Artifact — Spacecraft
//! Oracle: When this Spacecraft enters, create two 1/1 colorless Borg artifact creature tokens.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 7+.)
//! Oracle: 7+ | Flying
//! Oracle: Artifact creatures you control get +1/+1.
//! Set: TRC #110 — Star Trek Commander | Scryfall ID: 363d63ce-a9a8-46bf-b5e4-837aa10efca8 | Oracle ID: 9c14e21b-a733-4882-8e09-6dbfc85ac3aa
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CLASS_4_TACTICAL_CUBE,
    oracle_id = "9c14e21b-a733-4882-8e09-6dbfc85ac3aa",
    scryfall_id = "363d63ce-a9a8-46bf-b5e4-837aa10efca8",
    faces = &[face!(
        name = "Class-4 Tactical Cube",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
