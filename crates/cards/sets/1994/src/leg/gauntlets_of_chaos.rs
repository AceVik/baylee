//! Gauntlets of Chaos — {5} — Artifact
//! Oracle: {5}, Sacrifice this artifact: Exchange control of target artifact, creature, or land you control and target permanent an opponent controls that shares one of those types with it. If those permanents are exchanged this way, destroy all Auras attached to them.
//! Set: ME3 #196 — Masters Edition III | Scryfall ID: 8ea1b001-1eb6-4bb9-b8f3-1aaea14a9a13 | Oracle ID: 924ef69f-9977-439c-ace9-353b429e4be6
// IMPLEMENTED — exchange of two same-typed permanents; the Auras on both are destroyed only if the exchange happened.

use baylee_cards_dsl::prelude::*;

/// The three types the exchange is about (artifact, creature, land).
const EXCHANGE_TYPES: TypeSet = TypeSet::ARTIFACT
    .union(TypeSet::CREATURE)
    .union(TypeSet::LAND);

/// "target artifact, creature, or land you control"
const YOUR_EXCHANGE_PERMANENT: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::Or(&[Filter::ARTIFACT, Filter::CREATURE, Filter::LAND]),
]);

/// "target permanent an opponent controls that shares one of those types with
/// it" (the first target is the reference).
const OPPONENT_PERMANENT_SHARING_A_TYPE: Filter = Filter::And(&[
    Filter::ControlledByOpponent,
    Filter::SharesTypeWithReference(EXCHANGE_TYPES),
]);

card!(
    index = index::GAUNTLETS_OF_CHAOS,
    oracle_id = "924ef69f-9977-439c-ace9-353b429e4be6",
    scryfall_id = "8ea1b001-1eb6-4bb9-b8f3-1aaea14a9a13",
    faces = &[face!(
        name = "Gauntlets of Chaos",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{5}", SacrificeSelf),
        &[
            Effect::ExchangeControl,
            // "If those permanents are exchanged this way": both changed hands.
            Effect::IfCondition {
                condition: Condition::AmountAtLeast(Amount::ThisWay(ThisWay::ControlChanged), 2),
                then: &[Effect::DestroyAurasOnTargets],
                otherwise: &[],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&YOUR_EXCHANGE_PERMANENT))),
        second_targets = Some(TargetReq::one(TargetSpec::Object(
            &OPPONENT_PERMANENT_SHARING_A_TYPE
        ))),
    )],
);
