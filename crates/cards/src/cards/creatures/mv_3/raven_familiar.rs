//! Raven Familiar — {2}{U} — Creature — Bird
//! Oracle: Flying
//! Oracle: Echo {2}{U} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Oracle: When this creature enters, look at the top three cards of your library. Put one of them into your hand and the rest on the bottom of your library in any order.
//! Set: C13 #55 — Commander 2013 | Scryfall ID: 8b60227d-74de-4f2a-b281-3512dd77ffe8 | Oracle ID: 47aabd9d-0f68-490a-a2ae-5cd9ef689be6
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RAVEN_FAMILIAR,
    oracle_id = "47aabd9d-0f68-490a-a2ae-5cd9ef689be6",
    scryfall_id = "8b60227d-74de-4f2a-b281-3512dd77ffe8",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Raven Familiar",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD],
        power = Some(1),
        toughness = Some(2),
    ),],
    abilities = &[
        AbilityDef::Echo {
            cost: mana!("{2}{U}"),
        },
        triggered!(
            Trigger::ETB,
            &[Effect::LookAtTopPick {
                count: Amount::Fixed(3),
                pick: 1,
                random: false,
            }],
        ),
    ],
);
