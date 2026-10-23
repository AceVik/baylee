//! `cards/creatures/mv_4/dauthi_mindripper.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn dauthi_mindripper() -> CardIndex {
    card_index("ecae5eba-8407-4c71-aeae-ab10fb1e5226")
}

/// Dauthi Mindripper (`ecae5eba-8407-4c71-aeae-ab10fb1e5226`) — {3}{B} Creature — Dauthi Minion:
/// "Shadow. Whenever this creature attacks and isn't blocked, you may sacrifice it. If you do, defending player discards three cards."
#[test]
fn dauthi_mindripper_sacrifices_when_unblocked_to_make_defender_discard_three() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dauthi_mindripper()])
        .hand(1, &[forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let ripper = on_battlefield(&engine, P0, dauthi_mindripper()).unwrap();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P1)).len();
    assert!(hand_before >= 3);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ripper, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    // Accept the optional sacrifice trigger.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();

    // Defending player chooses 3 cards to discard.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards");
    };
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: options[..3].to_vec(),
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, dauthi_mindripper()).is_none());
    assert!(in_graveyard(&engine, P0, dauthi_mindripper()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P1)).len(),
        hand_before - 3
    );
}

#[test]
fn dauthi_mindripper_may_decline_and_does_not_trigger_when_blocked() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::generated::index;
    for blocked in [false, true] {
        let mut e = lk::game(
            SEED,
            &[dauthi_mindripper()],
            &[index::DAUTHI_GHOUL],
            &[],
            &[forest(), forest(), forest()],
        );
        let ripper = lk::obj(&e, P0, dauthi_mindripper());
        let blocker = lk::obj(&e, P1, index::DAUTHI_GHOUL);
        lk::attack(&mut e, &[ripper]);
        let blocking_pair = [(blocker, ripper)];
        lk::block(&mut e, if blocked { &blocking_pair } else { &[] });
        if !blocked {
            pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
            e.apply(P0, PlayerAction::YesNo(false)).unwrap();
        }
        lk::settle(&mut e);
        assert_eq!(e.state().zones.list(ZoneLocation::Hand(P1)).len(), 3);
        if !blocked {
            assert!(on_battlefield(&e, P0, dauthi_mindripper()).is_some());
        }
    }
}
