//! U.S.S. Defiant, Mirror Migrant — {2} — Legendary Artifact — Spacecraft
//! Oracle: Flash
//! Oracle: When U.S.S. Defiant enters, it and up to one target creature you control phase out. (Treat them and anything attached to them as though they don't exist until your next turn.)
//! Oracle: Station
//! Oracle: 8+ | Flying
//! Set: TRC #124 — Star Trek Commander | Scryfall ID: 6c347d57-4a92-42eb-b703-d4bec298e94b | Oracle ID: c7c529c4-08eb-480b-a699-0d783215419b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::U_S_S_DEFIANT_MIRROR_MIGRANT,
    oracle_id = "c7c529c4-08eb-480b-a699-0d783215419b",
    scryfall_id = "6c347d57-4a92-42eb-b703-d4bec298e94b",
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "U.S.S. Defiant, Mirror Migrant",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
