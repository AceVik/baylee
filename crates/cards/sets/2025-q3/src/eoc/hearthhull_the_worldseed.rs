//! Hearthhull, the Worldseed — {1}{B}{R}{G} — Legendary Artifact — Spacecraft
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 8+.)
//! Oracle: 2+ | {1}, {T}, Sacrifice a land: Draw two cards. You may play an additional land this turn.
//! Oracle: 8+ | Flying, vigilance, haste
//! Oracle: Whenever you sacrifice a land, each opponent loses 2 life.
//! Set: EOC #1 — Edge of Eternities Commander | Scryfall ID: 6b913e79-a9f6-44d4-bb37-c4f6f66b8151 | Oracle ID: 8d6f82d9-5c5b-4c41-8857-c0936f2d894c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HEARTHHULL_THE_WORLDSEED,
    oracle_id = "8d6f82d9-5c5b-4c41-8857-c0936f2d894c",
    scryfall_id = "6b913e79-a9f6-44d4-bb37-c4f6f66b8151",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green, Color::Red]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Hearthhull, the Worldseed",
        mana_cost = mana!("{1}{B}{R}{G}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(6),
        toughness = Some(7),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
