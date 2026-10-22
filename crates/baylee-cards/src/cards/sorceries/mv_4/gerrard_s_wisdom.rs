//! Gerrard's Wisdom — {2}{W}{W} — Sorcery
//! Oracle: You gain 2 life for each card in your hand.
//! Set: 7ED #16 — Seventh Edition | Scryfall ID: e43378f0-b518-499e-9950-c583bc2f11f8 | Oracle ID: 3e30e8f2-f437-4620-a3bc-dd9c29ae6570
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GERRARD_S_WISDOM,
    oracle_id = "3e30e8f2-f437-4620-a3bc-dd9c29ae6570",
    scryfall_id = "e43378f0-b518-499e-9950-c583bc2f11f8",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Gerrard's Wisdom",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::GainLife {
        amount: Amount::Times {
            base: &Amount::HandSize(PlayerRel::You),
            factor: 2,
        },
    }])],
);
