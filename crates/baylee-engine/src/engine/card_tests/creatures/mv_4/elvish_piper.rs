//! `cards/creatures/mv_4/elvish_piper.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn elvish_piper() -> CardIndex {
    card_index("872ef617-7cdb-4a7a-85f5-efc9e12bece5")
}

#[test]
fn elvish_piper_puts_creature_card_from_hand_onto_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[earth_elemental(), forest()])
        .battlefield(0, &[elvish_piper(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elem_hand = in_hand(&engine, p0, earth_elemental()).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, elvish_piper(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    let land = in_hand(&engine, p0, forest()).unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending() else {
        panic!("creature choice")
    };
    assert!(options.contains(&elem_hand));
    assert!(!options.contains(&land));
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![elem_hand],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, earth_elemental()).is_some());
}

#[test]
fn elvish_piper_may_put_no_creature_into_play() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(SEED, &[elvish_piper()], &[], &[earth_elemental()], &[]);
    lk::float(&mut engine, lk::P0, &[(ManaColor::Green, 1)]);
    activate(&mut engine, lk::P0, elvish_piper(), 0);
    let Pending::ChooseCards { min, .. } = pass_to_card_choice(&mut engine) else {
        unreachable!()
    };
    assert_eq!(min, 0);
    engine
        .apply(lk::P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, lk::P0, earth_elemental()).is_some());
    assert!(is_tapped(&engine, lk::obj(&engine, lk::P0, elvish_piper())));
}
