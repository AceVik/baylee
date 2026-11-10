//! Goblin Recruiter — {1}{R} — Creature — Goblin
//! Oracle: When this creature enters, search your library for any number of Goblin cards, reveal them, then shuffle and put those cards on top in any order.
//! Set: 6ED #186 — Classic Sixth Edition | Scryfall ID: 61bd1548-2ffa-4705-ba88-913f37d4ce92 | Oracle ID: 792e1072-00bf-4f63-b956-5f20cdf09f11
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static GOBLIN_CARD: Filter = Filter::HasSubtype(subtypes::creature::GOBLIN);

card!(
    index = index::GOBLIN_RECRUITER,
    oracle_id = "792e1072-00bf-4f63-b956-5f20cdf09f11",
    scryfall_id = "61bd1548-2ffa-4705-ba88-913f37d4ce92",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Recruiter",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::SearchLibrary {
            filter: &GOBLIN_CARD,
            finds: &[Find::TOP_OF_LIBRARY.any_number()],
            optional: true,
        }],
    )],
);
