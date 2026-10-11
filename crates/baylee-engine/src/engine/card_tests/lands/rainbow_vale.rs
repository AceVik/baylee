//! `cards/lands/rainbow_vale.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rainbow Vale: "{T}: Add one mana of any color. An opponent gains control of this land at the beginning of the next end step."
/// Under `Coverage::Partial`, the end-step control change trigger is omitted.
/// Activating the land prompts for a color choice; selecting Green adds {G} to the mana pool.
#[test]
fn rainbow_vale_adds_chosen_color_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(138, forest())
        .battlefield(0, &[rainbow_vale()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vale = on_battlefield(&engine, p0, rainbow_vale()).expect("Vale deployed");
    activate(&mut engine, p0, rainbow_vale(), 0);

    let Pending::ChooseColor { player, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, vale));
}

#[test]
fn rainbow_vale_defers_an_untargeted_opponent_choice_until_the_end_step() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    for blink in [false, true] {
        let mut e = Duel::table(SEED, forest(), 3)
            .battlefield(0, &[rainbow_vale()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let vale = on_battlefield(&e, p0, rainbow_vale()).unwrap();
        activate(&mut e, p0, rainbow_vale(), 0);
        e.apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
            .unwrap();
        assert!(stack_is_empty(&e));
        assert_eq!(e.state().object(vale).unwrap().controller, p0);
        if blink {
            let state = e.dev_state_mut(p0).unwrap();
            state
                .move_object(
                    vale,
                    ZoneLocation::Exile(p0),
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .unwrap();
            state
                .move_object(
                    vale,
                    ZoneLocation::Battlefield,
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .unwrap();
        }
        pass_until(&mut e, |e| e.state().turn.phase == Phase::Ending);
        if blink {
            pass_until(&mut e, stack_is_empty);
            assert_eq!(e.state().object(vale).unwrap().controller, p0);
        } else {
            pass_until(&mut e, |e| {
                matches!(e.pending(), Pending::ChoosePlayer { .. })
            });
            let Pending::ChoosePlayer { options, .. } = e.pending() else {
                unreachable!()
            };
            assert_eq!(options, &vec![p1, p2]);
            let hash = e.state().snapshot_hash();
            assert!(e.apply(p0, PlayerAction::ChoosePlayer(p0)).is_err());
            assert_eq!(hash, e.state().snapshot_hash());
            e.apply(p0, PlayerAction::ChoosePlayer(p2)).unwrap();
            pass_until(&mut e, stack_is_empty);
            assert_eq!(e.state().object(vale).unwrap().controller, p2);
        }
    }
}
