//! Paupers' Cage — {3} — Artifact
//! Oracle: At the beginning of each opponent's upkeep, if that player has two or fewer cards in hand, this artifact deals 2 damage to that player.
//! Set: MIR #314 — Mirage | Scryfall ID: 3411c350-a940-4172-b558-f5dc44b4fb33 | Oracle ID: 38ec5b13-b143-4818-9243-b1715dab3bef
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static AT_LEAST_THREE_IN_HAND: Condition =
    Condition::AmountAtLeast(Amount::HandSize(PlayerRel::ActivePlayer), 3);

card!(
    index = index::PAUPERS_CAGE,
    oracle_id = "38ec5b13-b143-4818-9243-b1715dab3bef",
    scryfall_id = "3411c350-a940-4172-b558-f5dc44b4fb33",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Paupers' Cage",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::Opponent,
        },
        &[Effect::DealDamage {
            amount: Amount::Fixed(2),
            target: TargetSpec::Player(PlayerRel::ActivePlayer),
        }],
        condition = Some(Condition::Not(&AT_LEAST_THREE_IN_HAND)),
    )],
);
