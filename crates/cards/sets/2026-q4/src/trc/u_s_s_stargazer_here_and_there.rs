//! U.S.S. Stargazer, Here and There — {3} — Legendary Artifact — Spacecraft
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 6+.)
//! Oracle: 6+ | Flying
//! Oracle: Picard Maneuver — Whenever U.S.S. Stargazer attacks, create a tapped and attacking 4/3 colorless Spacecraft Illusion artifact creature token with flying. Exile that token at end of combat.
//! Set: TRC #129 — Star Trek Commander | Scryfall ID: b63286ae-127b-4eb0-870f-f105decf575a | Oracle ID: a5b72c5d-4c73-49cd-9fc5-855856807ff2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::U_S_S_STARGAZER_HERE_AND_THERE,
    oracle_id = "a5b72c5d-4c73-49cd-9fc5-855856807ff2",
    scryfall_id = "b63286ae-127b-4eb0-870f-f105decf575a",
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "U.S.S. Stargazer, Here and There",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(4),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
