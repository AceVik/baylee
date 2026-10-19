//! Cleansing — {W}{W}{W} — Sorcery
//! Oracle: For each land, destroy that land unless any player pays 1 life.
//! Set: DRK #4 — The Dark | Scryfall ID: fc1973a3-1410-4c6d-9b09-bd9d18646a1e | Oracle ID: 46b83657-1699-4162-a4bb-b792da396fe6

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CLEANSING,
    oracle_id = "46b83657-1699-4162-a4bb-b792da396fe6",
    scryfall_id = "fc1973a3-1410-4c6d-9b09-bd9d18646a1e",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Cleansing",
        mana_cost = mana!("{W}{W}{W}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::DestroyEachUnlessAnyPlayerPaysLife {
        filter: &Filter::LAND,
        life: 1,
    }])],
);
