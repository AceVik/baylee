//! Misers' Cage — {3} — Artifact
//! Oracle: At the beginning of each opponent's upkeep, if that player has five or more cards in hand, this artifact deals 2 damage to that player.
//! Set: MIR #311 — Mirage | Scryfall ID: f4c00691-26b1-4e7a-bdf5-4b15f0eb45ce | Oracle ID: 72a1e724-eb32-4781-b44d-3b1ed02a5a8b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MISERS_CAGE,
    oracle_id = "72a1e724-eb32-4781-b44d-3b1ed02a5a8b",
    scryfall_id = "f4c00691-26b1-4e7a-bdf5-4b15f0eb45ce",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Misers' Cage",
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
        condition = Some(Condition::AmountAtLeast(
            Amount::HandSize(PlayerRel::ActivePlayer),
            5,
        )),
    )],
);
