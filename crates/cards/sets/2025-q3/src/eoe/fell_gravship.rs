//! Fell Gravship — {2}{B} — Artifact — Spacecraft
//! Oracle: When this Spacecraft enters, mill three cards, then return a creature or Spacecraft card from your graveyard to your hand.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 8+.)
//! Oracle: 8+ | Flying, lifelink
//! Set: EOE #101 — Edge of Eternities | Scryfall ID: e94b130d-3547-43c5-a319-5ebc571c2e2d | Oracle ID: 1a82be68-3b74-4dfc-9068-3abea61db709
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FELL_GRAVSHIP,
    oracle_id = "1a82be68-3b74-4dfc-9068-3abea61db709",
    scryfall_id = "e94b130d-3547-43c5-a319-5ebc571c2e2d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Fell Gravship",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(3),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
