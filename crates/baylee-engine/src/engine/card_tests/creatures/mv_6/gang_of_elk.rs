#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn gang_of_elk() -> CardIndex {
    card_index("b99d6d51-3ead-4b81-8e0c-f458619ebb24")
}

#[test]
fn gang_of_elk_gets_plus_two_plus_two_for_each_creature_blocking_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[gang_of_elk()])
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .battlefield(1, &[grizzly_bears(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, gang_of_elk());
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let elk = on_battlefield(&engine, p0, gang_of_elk()).unwrap();
    let b1 = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let b2 = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    let _ = attack_and_collect_blocks(&mut engine, elk, p1);
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(b1, elk), (b2, elk)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elk), (9, 8));
    kit::drive(
        &mut engine,
        |e| {
            e.state().turn.active == p1
                && e.state().turn.phase == Phase::FirstMain
                && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
        },
        |_, pending| match pending {
            Pending::ChooseNumber {
                max,
                reason: crate::choice::NumberPrompt::CombatDamage { .. },
                ..
            } => Some(PlayerAction::ChooseNumber(*max)),
            _ => None,
        },
    );
    assert_eq!(pt(&engine, elk), (5, 4));
}
