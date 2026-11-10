//! Imperial Recruiter — {2}{R} — Creature — Human Advisor
//! Oracle: When this creature enters, search your library for a creature card with power 2 or less, reveal it, put it into your hand, then shuffle.
//! Set: MH2 #281 — Modern Horizons 2 | Scryfall ID: 05bd329b-5707-42fc-af1c-084cc604e805 | Oracle ID: 4d6a1391-817a-4ddc-840d-886b138eeb3f
// IMPLEMENTED — an enters trigger searching for a creature card with power
// 2 or less; a find that goes to a hand is revealed.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::IMPERIAL_RECRUITER,
    oracle_id = "4d6a1391-817a-4ddc-840d-886b138eeb3f",
    scryfall_id = "05bd329b-5707-42fc-af1c-084cc604e805",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Imperial Recruiter",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ADVISOR],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::SearchLibrary {
            filter: &Filter::And(&[Filter::CREATURE, Filter::PowerAtMost(2)]),
            finds: &[Find::HAND],
            optional: false,
        }]
    )],
);
