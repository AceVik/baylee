//! Infinite Guideline Station — {W}{U}{B}{R}{G} — Legendary Artifact — Spacecraft
//! Oracle: When Infinite Guideline Station enters, create a tapped 2/2 colorless Robot artifact creature token for each multicolored permanent you control.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 12+.)
//! Oracle: 12+ | Flying
//! Oracle: Whenever Infinite Guideline Station attacks, draw a card for each multicolored permanent you control.
//! Set: EOE #219 — Edge of Eternities | Scryfall ID: 5688894a-bbec-476b-ae2e-94000be258d0 | Oracle ID: cda39514-86c9-483e-8d95-fd08138ce33a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::INFINITE_GUIDELINE_STATION,
    oracle_id = "cda39514-86c9-483e-8d95-fd08138ce33a",
    scryfall_id = "5688894a-bbec-476b-ae2e-94000be258d0",
    color_identity = ColorSet::from_slice(&[
        Color::Black,
        Color::Green,
        Color::Red,
        Color::Blue,
        Color::White
    ]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Infinite Guideline Station",
        mana_cost = mana!("{W}{U}{B}{R}{G}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(7),
        toughness = Some(15),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
