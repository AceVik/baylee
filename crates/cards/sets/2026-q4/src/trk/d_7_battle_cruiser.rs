//! D-7 Battle Cruiser — {1}{R} — Artifact — Spacecraft
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 8+.)
//! Oracle: 3+ | At the beginning of combat on your turn, create a 2/1 red Klingon Warrior creature token with haste. Sacrifice it at the beginning of the next end step.
//! Oracle: 8+ | Flying
//! Set: TRK #145 — Star Trek | Scryfall ID: 6c22998d-7bd7-433d-95b8-50f2fd8b7b7f | Oracle ID: 5e696f9a-425c-470a-870e-10d2ba3bb27d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::D_7_BATTLE_CRUISER,
    oracle_id = "5e696f9a-425c-470a-870e-10d2ba3bb27d",
    scryfall_id = "6c22998d-7bd7-433d-95b8-50f2fd8b7b7f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "D-7 Battle Cruiser",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
