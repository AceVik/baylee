//! Scapegoat — {W} — Instant
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: Return any number of target creatures you control to their owner's hand.
//! Set: STH #14 — Stronghold | Scryfall ID: 20454d36-d98d-4421-b3aa-d2dd4b368d84 | Oracle ID: 04eb4fc0-0cc5-44f1-af5a-44579125d5c2
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SCAPEGOAT,
    oracle_id = "04eb4fc0-0cc5-44f1-af5a-44579125d5c2",
    scryfall_id = "20454d36-d98d-4421-b3aa-d2dd4b368d84",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Scapegoat",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
        mandatory_additional_costs = &[CostPart::Sacrifice(&Filter::YOUR_CREATURE)],
    ),],
    abilities = &[spell!(
        &[Effect::ReturnToHand {
            target: TargetSpec::Object(&Filter::YOUR_CREATURE),
        }],
        targets = Some(TargetReq::up_to(
            TargetSpec::Object(&Filter::YOUR_CREATURE),
            255
        )),
    ),],
);
