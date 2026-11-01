//! Wheel of Torture — {3} — Artifact
//! Oracle: At the beginning of each opponent's upkeep, this artifact deals X damage to that player, where X is 3 minus the number of cards in their hand.
//! Set: ULG #138 — Urza's Legacy | Scryfall ID: 87edc873-169a-4cd3-8a94-84b5810b5ed8 | Oracle ID: 7a3e6e84-a8c2-472d-99fb-7cd14850b5ce
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WHEEL_OF_TORTURE,
    oracle_id = "7a3e6e84-a8c2-472d-99fb-7cd14850b5ce",
    scryfall_id = "87edc873-169a-4cd3-8a94-84b5810b5ed8",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Wheel of Torture",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::Opponent,
        },
        &[Effect::DealDamage {
            amount: Amount::Difference {
                left: &Amount::Fixed(3),
                right: &Amount::CountOf {
                    filter: &Filter::Any,
                    zone: ZoneSel::HandActivePlayer,
                },
            },
            target: TargetSpec::Player(PlayerRel::ActivePlayer),
        }],
    )],
);
