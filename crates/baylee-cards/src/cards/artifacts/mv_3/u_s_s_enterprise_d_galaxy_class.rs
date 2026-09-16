//! U.S.S. Enterprise-D, Galaxy-Class — {3} — Legendary Artifact — Spacecraft
//! Oracle: Whenever one or more charge counters are put on U.S.S. Enterprise-D for the first time each turn, exile the top card of your library. You may play that card this turn.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 7+.)
//! Oracle: 7+ | Flying, vigilance
//! Set: TRK #273 — Star Trek | Scryfall ID: 057a4413-6a17-491e-bfd7-6cd427b1a442 | Oracle ID: d95af032-3efd-40c7-8229-ade9d974934f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::U_S_S_ENTERPRISE_D_GALAXY_CLASS,
    oracle_id = "d95af032-3efd-40c7-8229-ade9d974934f",
    scryfall_id = "057a4413-6a17-491e-bfd7-6cd427b1a442",
    faces = &[face!(
        name = "U.S.S. Enterprise-D, Galaxy-Class",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(4),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
