//! Legerdemain — {2}{U}{U} — Sorcery
//! Oracle: Exchange control of target artifact or creature and another target permanent that shares one of those types with it. (This effect lasts indefinitely.)
//! Set: TPR #57 — Tempest Remastered | Scryfall ID: 921f5746-496d-4c73-bc43-fa8601c2f1e9 | Oracle ID: 107d5f67-1395-444f-b612-c5cc770df956
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

const EXCHANGE_TYPES: TypeSet = TypeSet::ARTIFACT.union(TypeSet::CREATURE);
const FIRST_TARGET: Filter = Filter::Or(&[Filter::ARTIFACT, Filter::CREATURE]);
const SECOND_TARGET: Filter = Filter::And(&[
    Filter::Not(&Filter::Reference),
    Filter::SharesTypeWithReference(EXCHANGE_TYPES),
]);

card!(
    index = index::LEGERDEMAIN,
    oracle_id = "107d5f67-1395-444f-b612-c5cc770df956",
    scryfall_id = "921f5746-496d-4c73-bc43-fa8601c2f1e9",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Legerdemain",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::ExchangeControl],
        targets = Some(TargetReq::one(TargetSpec::Object(&FIRST_TARGET))),
        second_targets = Some(TargetReq::one(TargetSpec::Object(&SECOND_TARGET))),
    )],
);
