//! Animal Magnetism — {4}{G} — Sorcery
//! Oracle: Reveal the top five cards of your library. An opponent chooses a creature card from among them. Put that card onto the battlefield and the rest into your graveyard.
//! Set: CMM #271 — Commander Masters | Scryfall ID: c64860ab-9b10-4587-a1db-05b3797df3a7 | Oracle ID: a1bdf7f1-e3f7-4a13-9cf0-a2394df6cc74
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ANIMAL_MAGNETISM,
    oracle_id = "a1bdf7f1-e3f7-4a13-9cf0-a2394df6cc74",
    scryfall_id = "c64860ab-9b10-4587-a1db-05b3797df3a7",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Animal Magnetism",
        mana_cost = mana!("{4}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
