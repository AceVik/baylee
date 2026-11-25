//! Sarcophagus, Ship of the Dead — {2}{R} — Legendary Artifact — Spacecraft
//! Oracle: Other creatures you control get +1/+0.
//! Oracle: Whenever a creature dies, if Sarcophagus isn't a creature, put a charge counter on it.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 10+.)
//! Oracle: 10+ | Flying
//! Set: TRK #170 — Star Trek | Scryfall ID: b5ea32b5-aa58-4f87-ac2e-4e3b8dd4a752 | Oracle ID: 22b86863-c992-405b-949a-bdf8859884ef
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SARCOPHAGUS_SHIP_OF_THE_DEAD,
    oracle_id = "22b86863-c992-405b-949a-bdf8859884ef",
    scryfall_id = "b5ea32b5-aa58-4f87-ac2e-4e3b8dd4a752",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Sarcophagus, Ship of the Dead",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
