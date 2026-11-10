//! Summer Bloom — {1}{G} — Sorcery
//! Oracle: You may play up to three additional lands this turn.
//! Set: 2X2 #160 — Double Masters 2022 | Scryfall ID: 7383f71d-3f51-4632-9ea0-27d47fa2fa50 | Oracle ID: e5df4597-1647-4ac2-bdb3-a517598d1431
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SUMMER_BLOOM,
    oracle_id = "e5df4597-1647-4ac2-bdb3-a517598d1431",
    scryfall_id = "7383f71d-3f51-4632-9ea0-27d47fa2fa50",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Summer Bloom",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::continuous(
        &Filter::Any,
        Modifier::ExtraLandDrops(3),
        Duration::UntilEndOfTurn,
    )])],
);
