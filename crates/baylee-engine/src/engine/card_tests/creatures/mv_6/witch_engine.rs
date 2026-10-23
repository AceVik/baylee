#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn witch_engine() -> CardIndex {
    card_index("48adccf6-adb4-49b2-b013-4dc5e0fc7bcb")
}

#[test]
fn witch_engine_taps_for_four_black_mana_and_gives_control_to_opponent() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[witch_engine()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, witch_engine(), 0);
    match engine.pending().clone() {
        Pending::ChooseTargets { .. } => {
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![p1],
                    },
                )
                .unwrap();
        }
        Pending::ChoosePlayer { .. } => {
            engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
        }
        other => panic!("unexpected pending: {other:?}"),
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, witch_engine()).is_some());
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        4
    );
}

#[test]
fn witch_engine_is_a_targeted_stack_ability_and_has_swampwalk() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(SEED, &[witch_engine()], &[swamp(), gray_ogre()], &[], &[]);
    let witch = lk::obj(&engine, p0, witch_engine());
    let ogre = lk::obj(&engine, p1, gray_ogre());
    let blocks = lk::attack(&mut engine, &[witch]);
    assert!(!lk::may_block(&blocks, ogre, witch));
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    activate(&mut engine, p0, witch_engine(), 0);
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p0]
                }
            )
            .is_err()
    );
    lk::aim(&mut engine, &[], &[p1]);
    assert!(!stack_is_empty(&engine));
    assert_eq!(
        lk::pool_of(&engine, p0, ManaColor::Black),
        0,
        "mana waits for resolution"
    );
    lk::settle(&mut engine);
    assert_eq!(lk::pool_of(&engine, p0, ManaColor::Black), 4);
    assert!(on_battlefield(&engine, p1, witch_engine()).is_some());
}
