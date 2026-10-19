//! Wand of Ith — {4} — Artifact
//! Oracle: {3}, {T}: Target player reveals a card at random from their hand. If it's a land card, that player discards it unless they pay 1 life. If it isn't a land card, the player discards it unless they pay life equal to its mana value. Activate only during your turn.
//! Set: DRK #114 — The Dark | Scryfall ID: 80c9070a-8c70-480e-a476-e00f8e2c71b9 | Oracle ID: 3ec1ccd7-4970-4032-be7a-9e76ca6a139e

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WAND_OF_ITH,
    oracle_id = "3ec1ccd7-4970-4032-be7a-9e76ca6a139e",
    scryfall_id = "80c9070a-8c70-480e-a476-e00f8e2c71b9",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Wand of Ith",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[AbilityDef::ActivatedConditional {
        cost: cost!("{3}", TapSelf),
        effects: &[Effect::WandOfIth],
        targets: Some(TargetReq::one(TargetSpec::AnyPlayer)),
        second_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        condition: Condition::YourTurn,
        limit: ActivationLimit::Unlimited,
        x_value: ActivationX::Any,
        cost_reduction: None,
    }],
);
