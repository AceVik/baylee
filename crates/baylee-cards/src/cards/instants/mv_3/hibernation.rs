//! Hibernation — {2}{U} — Instant
//! Oracle: Return all green permanents to their owners' hands.
//! Set: USG #79 — Urza's Saga | Scryfall ID: 68b7444c-fabb-4437-8db9-a1008ea09415 | Oracle ID: 633b541b-e37f-451a-be73-3398e61e0f75
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HIBERNATION,
    oracle_id = "633b541b-e37f-451a-be73-3398e61e0f75",
    scryfall_id = "68b7444c-fabb-4437-8db9-a1008ea09415",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hibernation",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::ReturnAllToHand {
        filter: &Filter::HasColor(ColorSet::from_slice(&[Color::Green])),
        opponents_only: false,
    }])],
);
