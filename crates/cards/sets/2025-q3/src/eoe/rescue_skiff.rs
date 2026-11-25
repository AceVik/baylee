//! Rescue Skiff — {5}{W} — Artifact — Spacecraft
//! Oracle: When this Spacecraft enters, return target creature or enchantment card from your graveyard to the battlefield.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 10+.)
//! Oracle: 10+ | Flying
//! Set: EOE #32 — Edge of Eternities | Scryfall ID: 6fe86bfb-c67d-4df9-88c9-f083091f4cda | Oracle ID: 112aaaf0-3301-4f79-9640-d36c75a0fb30
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RESCUE_SKIFF,
    oracle_id = "112aaaf0-3301-4f79-9640-d36c75a0fb30",
    scryfall_id = "6fe86bfb-c67d-4df9-88c9-f083091f4cda",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Rescue Skiff",
        mana_cost = mana!("{5}{W}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(5),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
