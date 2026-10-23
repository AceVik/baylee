//! Headlong Rush — {1}{R} — Instant
//! Oracle: Attacking creatures gain first strike until end of turn.
//! Set: USG #198 — Urza's Saga | Scryfall ID: 9f0db04c-5101-4a43-9109-3964ac66bdab | Oracle ID: cc3abacc-6643-49c5-8df8-5b56ab20fba5
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HEADLONG_RUSH,
    oracle_id = "cc3abacc-6643-49c5-8df8-5b56ab20fba5",
    scryfall_id = "9f0db04c-5101-4a43-9109-3964ac66bdab",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Headlong Rush",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::continuous(
        &Filter::And(&[Filter::CREATURE, Filter::Attacking]),
        Modifier::AddKeyword(KeywordSet::FIRST_STRIKE),
        Duration::UntilEndOfTurn,
    )])],
);
