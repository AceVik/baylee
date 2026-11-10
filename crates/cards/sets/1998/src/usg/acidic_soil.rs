//! Acidic Soil — {2}{R} — Sorcery
//! Oracle: Acidic Soil deals damage to each player equal to the number of lands they control.
//! Set: USG #172 — Urza's Saga | Scryfall ID: 790157c9-b1ed-4da5-9d50-e99e0dd807b7 | Oracle ID: 4ccc1ac5-db90-4fc9-9795-7afaf7466c3e
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ACIDIC_SOIL,
    oracle_id = "4ccc1ac5-db90-4fc9-9795-7afaf7466c3e",
    scryfall_id = "790157c9-b1ed-4da5-9d50-e99e0dd807b7",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Acidic Soil",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::DealDamage {
        amount: Amount::ControlledCount {
            who: PlayerRel::Recipient,
            filter: &Filter::LAND,
        },
        target: TargetSpec::Player(PlayerRel::EachPlayer),
    }]),],
);
