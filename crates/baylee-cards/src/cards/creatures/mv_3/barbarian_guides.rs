//! Barbarian Guides — {2}{R} — Creature — Human Barbarian
//! Oracle: {2}{R}, {T}: Choose a land type. Target creature you control gains snow landwalk of the chosen type until end of turn. Return that creature to its owner's hand at the beginning of the next end step. (It can't be blocked as long as defending player controls a snow land of that type.)
//! Set: ICE #174 — Ice Age | Scryfall ID: fe65a045-dacb-4392-bcb6-843394ef98c9 | Oracle ID: ef8c324a-582b-401e-add1-1985c5596908

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: "Choose a land type. Target creature you control gains snow landwalk of the chosen type until end of turn" —
// Effect::ChooseBasicLandTypeAndGrantLandwalk grants basic landwalk rather than snow landwalk of the chosen type.
card!(
    index = index::BARBARIAN_GUIDES,
    oracle_id = "ef8c324a-582b-401e-add1-1985c5596908",
    scryfall_id = "fe65a045-dacb-4392-bcb6-843394ef98c9",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial("missing snow landwalk of a chosen land type"),
    faces = &[face!(
        name = "Barbarian Guides",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::BARBARIAN],
        power = Some(1),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{2}{R}", TapSelf),
        &[Effect::AtNextEndStep {
            effects: &[Effect::bounce(TargetSpec::EventObject)],
        }],
        target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE))
    )],
);
