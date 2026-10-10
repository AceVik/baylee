//! The Rack — {1} — Artifact
//! Oracle: As this artifact enters, choose an opponent.
//! Oracle: At the beginning of the chosen player's upkeep, this artifact deals X damage to that player, where X is 3 minus the number of cards in their hand.
//! Set: TSB #113 — Time Spiral Timeshifted | Scryfall ID: 696d1e25-5c25-4522-b085-90d49fe23a18 | Oracle ID: 3d873e1d-4fac-42c4-bb31-77e76099e1ef
// IMPLEMENTED — choose an opponent on entry; at their upkeep, 3 minus the
// cards in their hand (Amount::ConstantMinus), as Black Vise reads it.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THE_RACK,
    oracle_id = "3d873e1d-4fac-42c4-bb31-77e76099e1ef",
    scryfall_id = "696d1e25-5c25-4522-b085-90d49fe23a18",
    faces = &[face!(
        name = "The Rack",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
        enter_modifiers = &[EnterModifier::ChooseOpponent],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::StepBeginChosenOpponent {
            step: StepKind::Upkeep
        },
        &[Effect::DealDamage {
            amount: Amount::ConstantMinus {
                constant: 3,
                subtract: &Amount::CountOf {
                    filter: &Filter::Any,
                    zone: ZoneSel::HandActivePlayer
                },
            },
            target: TargetSpec::Player(PlayerRel::ActivePlayer),
        }]
    )],
);
