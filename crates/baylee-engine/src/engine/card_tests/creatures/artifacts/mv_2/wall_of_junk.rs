#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn wall_of_junk() -> CardIndex {
    card_index("22b0de2d-47a8-4269-ba21-4bc2b4b10b40")
}

#[test]
fn wall_of_junk_returns_to_hand_at_end_of_combat_when_blocking() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[wall_of_junk()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let wall = on_battlefield(&engine, p0, wall_of_junk()).expect("wall on bf");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("ogre on bf");
    let _ = attack_and_collect_blocks(&mut engine, ogre, p0);
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, ogre)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, wall_of_junk()).is_some(),
        "blocking schedules the return for later"
    );
    reach_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p0, wall_of_junk()).is_none());
    assert!(in_hand(&engine, p0, wall_of_junk()).is_some());
}

#[test]
fn wall_of_junk_cannot_attack_and_stays_when_it_did_not_block() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(SEED, &[wall_of_junk()], &[gray_ogre()], &[], &[]);
    let wall = lk::obj(&engine, p0, wall_of_junk());
    assert!(!lk::attackers_offered(&mut engine).contains(&wall));
    reach_their_main_phase(&mut engine, p1);
    let ogre = lk::obj(&engine, p1, gray_ogre());
    lk::attack(&mut engine, &[ogre]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert!(on_battlefield(&engine, p0, wall_of_junk()).is_some());
}
