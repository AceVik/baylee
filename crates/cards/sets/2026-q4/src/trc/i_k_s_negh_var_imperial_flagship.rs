//! I.K.S. Negh'Var, Imperial Flagship — {3} — Legendary Artifact — Spacecraft
//! Oracle: Klingons and Warriors you control have vigilance.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 10+.)
//! Oracle: 10+ | Flying, vigilance
//! Set: TRC #113 — Star Trek Commander | Scryfall ID: 1a9b48d4-3954-4073-a629-a4e31b9bc962 | Oracle ID: 373e2b58-ccf0-444b-aeb2-0b6e8d4e80e7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::I_K_S_NEGH_VAR_IMPERIAL_FLAGSHIP,
    oracle_id = "373e2b58-ccf0-444b-aeb2-0b6e8d4e80e7",
    scryfall_id = "1a9b48d4-3954-4073-a629-a4e31b9bc962",
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "I.K.S. Negh'Var, Imperial Flagship",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(6),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
