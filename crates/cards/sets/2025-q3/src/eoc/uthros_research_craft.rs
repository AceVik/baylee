//! Uthros Research Craft — {2}{U} — Artifact — Spacecraft
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 12+.)
//! Oracle: 3+ | Whenever you cast an artifact spell, draw a card. Put a charge counter on this Spacecraft.
//! Oracle: 12+ | Flying
//! Oracle: This Spacecraft gets +1/+0 for each artifact you control.
//! Set: EOC #7 — Edge of Eternities Commander | Scryfall ID: 6ade097b-349f-4876-bdc8-7bcd051e9952 | Oracle ID: e1c9783a-1d1b-40d7-872e-0ca11b229ce6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::UTHROS_RESEARCH_CRAFT,
    oracle_id = "e1c9783a-1d1b-40d7-872e-0ca11b229ce6",
    scryfall_id = "6ade097b-349f-4876-bdc8-7bcd051e9952",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Uthros Research Craft",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(0),
        toughness = Some(8),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
