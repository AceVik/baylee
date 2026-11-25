//! U.S.S. Excelsior, Cutting Edge — {3}{W} — Legendary Artifact — Spacecraft
//! Oracle: When U.S.S. Excelsior enters, destroy target tapped creature an opponent controls.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 8+.)
//! Oracle: 8+ | Flying
//! Set: TRK #44 — Star Trek | Scryfall ID: de62b452-3780-4eeb-8163-f44176495615 | Oracle ID: 9f4ae54b-f77d-43df-87d5-792ac0905058
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::U_S_S_EXCELSIOR_CUTTING_EDGE,
    oracle_id = "9f4ae54b-f77d-43df-87d5-792ac0905058",
    scryfall_id = "de62b452-3780-4eeb-8163-f44176495615",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "U.S.S. Excelsior, Cutting Edge",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
