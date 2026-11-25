//! Exploration Broodship — {G} — Artifact — Spacecraft
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 8+.)
//! Oracle: 3+ | You may play an additional land on each of your turns.
//! Oracle: 8+ | Flying
//! Oracle: Once during each of your turns, you may cast a permanent spell from your graveyard by sacrificing a land in addition to paying its other costs.
//! Set: EOC #14 — Edge of Eternities Commander | Scryfall ID: 61596da3-2a0e-47d9-b707-8db4cce2479d | Oracle ID: 8a83ecee-092b-456c-803f-a9aaa17c4da9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EXPLORATION_BROODSHIP,
    oracle_id = "8a83ecee-092b-456c-803f-a9aaa17c4da9",
    scryfall_id = "61596da3-2a0e-47d9-b707-8db4cce2479d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Exploration Broodship",
        mana_cost = mana!("{G}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
