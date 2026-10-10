//! `cards/enchantments/mv_2/arguel_s_blood_fast.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn arguel_s_blood_fast_charges_two_life_for_its_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(400, forest())
        .battlefield(0, &[arguel_s_blood_fast(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before = library_size(&engine, p0);
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, arguel_s_blood_fast(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(library_size(&engine, p0), before - 1, "one card drawn");
    assert_eq!(
        engine.state().players[0].life,
        18,
        "and two life paid for it"
    );
}

#[test]
fn arguel_checks_life_again_then_its_temple_uses_sacrificed_toughness() {
    let p0 = PlayerId::new(0);
    for response_life in [5, 6] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[
                    arguel_s_blood_fast(),
                    baylee_core::generated::index::WALL_OF_WOOD,
                ],
            )
            .life(0, 5)
            .start();
        keep_mulligans(&mut e);
        pass_until(&mut e, |e| !stack_is_empty(e));
        e.dev_state_mut(p0).unwrap().players[0].life = response_life;
        if response_life == 6 {
            pass_until(&mut e, stack_is_empty);
            let source = on_battlefield(&e, p0, arguel_s_blood_fast()).unwrap();
            assert_eq!(e.state().object(source).unwrap().face_index, 0);
            continue;
        }
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        e.apply(p0, PlayerAction::YesNo(true)).unwrap();
        pass_until(&mut e, stack_is_empty);
        reach_main_phase(&mut e, p0);
        let source = on_battlefield(&e, p0, arguel_s_blood_fast()).unwrap();
        assert_eq!(e.state().object(source).unwrap().face_index, 1);
        let wall = on_battlefield(&e, p0, baylee_core::generated::index::WALL_OF_WOOD).unwrap();
        activate(&mut e, p0, arguel_s_blood_fast(), 1);
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wall],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().players[0].life, 8);
    }
}
