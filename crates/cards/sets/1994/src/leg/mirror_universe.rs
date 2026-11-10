//! Mirror Universe — {6} — Artifact
//! Oracle: {T}, Sacrifice this artifact: Exchange life totals with target opponent. Activate only during your upkeep.
//! Set: ME1 #159 — Masters Edition | Scryfall ID: 0f9cbeaf-3456-4a87-ac75-e7658ccbd97f | Oracle ID: 8d231d2f-5274-4b55-8262-9eb95654d183
// IMPLEMENTED — {T}, sacrifice: exchange life totals with target opponent, only during your upkeep.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MIRROR_UNIVERSE,
    oracle_id = "8d231d2f-5274-4b55-8262-9eb95654d183",
    scryfall_id = "0f9cbeaf-3456-4a87-ac75-e7658ccbd97f",
    faces = &[face!(
        name = "Mirror Universe",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!(TapSelf, SacrificeSelf),
        &[Effect::ExchangeLifeTotals {
            with: PlayerRel::Chosen,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
        condition = Some(Condition::All(&[
            Condition::YourTurn,
            Condition::DuringStep(StepKind::Upkeep),
        ])),
    )],
);
