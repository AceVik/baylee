//! Typhoon — {2}{G} — Sorcery
//! Oracle: Typhoon deals damage to each opponent equal to the number of Islands that player controls.
//! Set: LEG #209 — Legends | Scryfall ID: 254e0403-67d8-4e73-8d89-c901ebeba49f | Oracle ID: ef295a34-0325-49dd-87a6-546dde395082
// IMPLEMENTED — each opponent is dealt damage equal to the Islands that player controls, counted per opponent on resolution.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TYPHOON,
    oracle_id = "ef295a34-0325-49dd-87a6-546dde395082",
    scryfall_id = "254e0403-67d8-4e73-8d89-c901ebeba49f",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Typhoon",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(&[Effect::DealDamage {
        amount: Amount::ControlledCount {
            who: PlayerRel::Recipient,
            filter: &Filter::HasSubtype(subtypes::land::ISLAND),
        },
        target: TargetSpec::Player(PlayerRel::EachOpponent),
    }])],
);
