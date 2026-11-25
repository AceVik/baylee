//! The Seriema — {1}{W}{W} — Legendary Artifact — Spacecraft
//! Oracle: When The Seriema enters, search your library for a legendary creature card, reveal it, put it into your hand, then shuffle.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 7+.)
//! Oracle: 7+ | Flying
//! Oracle: Other tapped legendary creatures you control have indestructible.
//! Set: EOE #35 — Edge of Eternities | Scryfall ID: dec91ec3-42d3-4922-96f0-dbb50a576084 | Oracle ID: a6bcec1f-f515-4e63-9e84-8eb04cc582ff
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THE_SERIEMA,
    oracle_id = "a6bcec1f-f515-4e63-9e84-8eb04cc582ff",
    scryfall_id = "dec91ec3-42d3-4922-96f0-dbb50a576084",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "The Seriema",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
