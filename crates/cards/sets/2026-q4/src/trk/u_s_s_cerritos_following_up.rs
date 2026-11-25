//! U.S.S. Cerritos, Following Up — {5} — Legendary Artifact — Spacecraft
//! Oracle: When U.S.S. Cerritos enters, create two 1/1 red Officer creature tokens.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 6+.)
//! Oracle: 6+ | Flying
//! Set: TRK #272 — Star Trek | Scryfall ID: 76bf7521-5024-4fdc-9ea1-5e10f2647efb | Oracle ID: 472eac8f-2b82-4fa9-a40c-af11ec40f19e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::U_S_S_CERRITOS_FOLLOWING_UP,
    oracle_id = "472eac8f-2b82-4fa9-a40c-af11ec40f19e",
    scryfall_id = "76bf7521-5024-4fdc-9ea1-5e10f2647efb",
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "U.S.S. Cerritos, Following Up",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
