//! Black Vise — {1} — Artifact
//! Oracle: As this artifact enters, choose an opponent.
//! Oracle: At the beginning of the chosen player's upkeep, this artifact deals X damage to that player, where X is the number of cards in their hand minus 4.
//! Set: ME3 #191 — Masters Edition III | Scryfall ID: bce2259a-f4cb-4130-9c7e-130980a8df38 | Oracle ID: de7839fb-7040-48ab-a6d4-d1952972943d
// IMPLEMENTED — choose an opponent on entry; their upkeep counts their current hand.
// Behavior tests: baylee-engine/src/engine/card_tests/artifacts.rs.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLACK_VISE,
    oracle_id = "de7839fb-7040-48ab-a6d4-d1952972943d",
    scryfall_id = "bce2259a-f4cb-4130-9c7e-130980a8df38",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Black Vise",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
        enter_modifiers = &[EnterModifier::ChooseOpponent],
    ),],
    abilities = &[triggered!(
        Trigger::StepBeginChosenOpponent {
            step: StepKind::Upkeep
        },
        &[Effect::DealDamage {
            amount: Amount::SaturatingSub {
                base: &Amount::CountOf {
                    filter: &Filter::Any,
                    zone: ZoneSel::HandActivePlayer
                },
                subtract: 4,
            },
            target: TargetSpec::Player(PlayerRel::ActivePlayer),
        }]
    )],
);
