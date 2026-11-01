//! `cards/creatures/mv_4/thalakos_deceiver.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn thalakos_deceiver() -> CardIndex {
    card_index("c304777a-39a9-4a2a-8cec-8116afb34c5b")
}

/// Thalakos Deceiver (`c304777a-39a9-4a2a-8cec-8116afb34c5b`) — {3}{U} 1/1 Creature — Thalakos Wizard (Implemented):
/// "Shadow. Whenever this creature attacks and isn't blocked, you may sacrifice it. If you do, gain control of target creature."
#[test]
fn thalakos_deceiver_sacrifices_on_unblocked_attack_to_gain_control_of_target_creature() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[thalakos_deceiver()])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let deceiver = on_battlefield(&engine, P0, thalakos_deceiver()).unwrap();
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(deceiver, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        aim_at(&mut engine, P0, giant);
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. }) || stack_is_empty(e)
    });
    if let Pending::YesNo { player, .. } = engine.pending().clone() {
        engine.apply(player, PlayerAction::YesNo(true)).unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, thalakos_deceiver()).is_none());
    assert_eq!(engine.state().object(giant).unwrap().controller, P0);
}

#[test]
fn unblocked_sacrifice_can_be_declined() {
    let mut e = lk::game(SEED, &[thalakos_deceiver()], &[index::HILL_GIANT], &[], &[]);
    let deceiver = lk::obj(&e, P0, thalakos_deceiver());
    let giant = lk::obj(&e, P1, index::HILL_GIANT);
    lk::attackers_offered(&mut e);
    let options = lk::declare(&mut e, &[deceiver]);
    assert!(!lk::may_block(&options, giant, deceiver));
    lk::block(&mut e, &[]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut e, &[giant], &[]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, false);
    lk::settle(&mut e);
    assert!(on_battlefield(&e, P0, thalakos_deceiver()).is_some());
    assert_eq!(e.state().object(giant).unwrap().controller, P1);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, P1), 19);
}
