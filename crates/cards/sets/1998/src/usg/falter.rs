//! Falter — {1}{R} — Instant
//! Oracle: Creatures without flying can't block this turn.
//! Set: USG #184 — Urza's Saga | Scryfall ID: e279126c-0512-4ee6-ad83-6fdfc7ae46c5 | Oracle ID: d4b50749-a016-4aff-8d70-a1707cabf57b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FALTER,
    oracle_id = "d4b50749-a016-4aff-8d70-a1707cabf57b",
    scryfall_id = "e279126c-0512-4ee6-ad83-6fdfc7ae46c5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Falter",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::continuous(
        &Filter::And(&[
            Filter::CREATURE,
            Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING)),
        ]),
        Modifier::CantBlock(&Filter::Any),
        Duration::UntilEndOfTurn,
    )])],
);
