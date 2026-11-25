//! U.S.S. Defiant, Tough Little Ship — {6} — Legendary Artifact — Spacecraft
//! Oracle: When U.S.S. Defiant enters, destroy up to one target artifact or enchantment.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 5+.)
//! Oracle: 5+ | Flying, hexproof
//! Set: TRC #125 — Star Trek Commander | Scryfall ID: e92acccf-d5a6-4559-8acd-d24492b0649e | Oracle ID: 87b8fef3-f40e-44a9-afd5-31d3917b1d59
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::U_S_S_DEFIANT_TOUGH_LITTLE_SHIP,
    oracle_id = "87b8fef3-f40e-44a9-afd5-31d3917b1d59",
    scryfall_id = "e92acccf-d5a6-4559-8acd-d24492b0649e",
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "U.S.S. Defiant, Tough Little Ship",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(7),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
