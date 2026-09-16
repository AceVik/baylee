//! Eternal Witness — {1}{G}{G} — Creature — Human Shaman
//! Oracle: When this creature enters, you may return target card from your graveyard to your hand.
//! Set: CMM #286 — Commander Masters | Scryfall ID: 39704000-65d3-4d39-849e-a3b617376bbc | Oracle ID: 30b24e8e-3b0e-4d8e-90f3-f66eb7c1858c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ETERNAL_WITNESS,
    oracle_id = "30b24e8e-3b0e-4d8e-90f3-f66eb7c1858c",
    scryfall_id = "39704000-65d3-4d39-849e-a3b617376bbc",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Eternal Witness",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SHAMAN],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
