//! Uthros Scanship — {3}{U} — Artifact — Spacecraft
//! Oracle: When this Spacecraft enters, draw two cards, then discard a card.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 8+.)
//! Oracle: 8+ | Flying
//! Set: EOE #85 — Edge of Eternities | Scryfall ID: 1f93887f-35c5-472f-83d0-54227b3bd1d2 | Oracle ID: 7b4c37dc-8cb0-4870-929e-11c2a45952a2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::UTHROS_SCANSHIP,
    oracle_id = "7b4c37dc-8cb0-4870-929e-11c2a45952a2",
    scryfall_id = "1f93887f-35c5-472f-83d0-54227b3bd1d2",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Uthros Scanship",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
