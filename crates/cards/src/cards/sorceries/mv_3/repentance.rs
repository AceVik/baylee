//! Repentance — {2}{W} — Sorcery
//! Oracle: Target creature deals damage to itself equal to its power.
//! Set: TPR #25 — Tempest Remastered | Scryfall ID: 7d02c74a-8bc7-4a4d-830b-e59a7aed5722 | Oracle ID: 9a345a67-0c6c-4f6f-b3c9-d525199cdbfa
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REPENTANCE,
    oracle_id = "9a345a67-0c6c-4f6f-b3c9-d525199cdbfa",
    scryfall_id = "7d02c74a-8bc7-4a4d-830b-e59a7aed5722",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Repentance",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::DamageEqualToPower {
            dealer: TargetSlot::First,
            to: TargetSlot::First,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
