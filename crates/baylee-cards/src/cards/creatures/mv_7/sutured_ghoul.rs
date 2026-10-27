//! Sutured Ghoul — {4}{B}{B}{B} — Creature — Zombie
//! Oracle: Trample
//! Oracle: As this creature enters, exile any number of creature cards from your graveyard.
//! Oracle: Sutured Ghoul's power is equal to the total power of the exiled cards and its toughness is equal to their total toughness.
//! Set: M12 #112 — Magic 2012 | Scryfall ID: 8390d9b7-5adf-4039-8682-02bfba421ff9 | Oracle ID: 8b4715b0-63d9-4ff8-a667-21a762518687
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SUTURED_GHOUL,
    oracle_id = "8b4715b0-63d9-4ff8-a667-21a762518687",
    scryfall_id = "8390d9b7-5adf-4039-8682-02bfba421ff9",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Sutured Ghoul",
        mana_cost = mana!("{4}{B}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
