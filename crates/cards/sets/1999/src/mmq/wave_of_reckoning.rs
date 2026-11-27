//! Wave of Reckoning — {4}{W} — Sorcery
//! Oracle: Each creature deals damage to itself equal to its power.
//! Set: SOC #112 — Secrets of Strixhaven Commander | Scryfall ID: 7be0c25a-5e65-48e1-9560-9391111bb912 | Oracle ID: 24f7143c-d43a-4120-baa0-0235b15d8186
// IMPLEMENTED — iterates over creatures on the battlefield so each deals damage equal to its power to itself.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WAVE_OF_RECKONING,
    oracle_id = "24f7143c-d43a-4120-baa0-0235b15d8186",
    scryfall_id = "7be0c25a-5e65-48e1-9560-9391111bb912",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Wave of Reckoning",
        mana_cost = mana!("{4}{W}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::ForEachMatching {
        filter: &Filter::CREATURE,
        effects: &[Effect::EventObjectDealsDamageEqualToPower {
            target: TargetSpec::EventObject,
        }],
    }])],
);
