//! `cards/instants/mv_2/fend_off.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn fend_off() -> CardIndex {
    card_index("fa8f3827-8cd9-4896-ab0c-26fecacceb40")
}

#[test]
fn fend_off_prevents_combat_damage_dealt_by_target_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[fend_off()])
        .battlefield(0, &[gray_ogre(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, fend_off());
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);

    let _ = attack_and_collect_blocks(&mut engine, ogre, p1);
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });

    assert_eq!(engine.state().players[1].life, 20);
}

#[test]
fn cycling_discards_for_two_and_draws_one() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(SEED, &[], &[], &[fend_off()], &[]);
    lk::float(&mut engine, lk::P0, &[(ManaColor::Colorless, 2)]);
    let before = engine.state().zones.list(ZoneLocation::Hand(lk::P0)).len();
    activate(&mut engine, lk::P0, fend_off(), 1);
    assert!(in_graveyard(&engine, lk::P0, fend_off()).is_some());
    assert_eq!(lk::pool_total(&engine, lk::P0), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(lk::P0)).len(),
        before
    );
}

#[test]
fn fend_off_prevents_a_blockers_damage_and_expires_next_turn() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(
        SEED,
        &[gray_ogre()],
        &[earth_elemental()],
        &[fend_off()],
        &[],
    );
    let ogre = lk::obj(&engine, lk::P0, gray_ogre());
    let blocker = lk::obj(&engine, lk::P1, earth_elemental());
    lk::float(&mut engine, lk::P0, &[(ManaColor::White, 2)]);
    lk::cast_at(&mut engine, lk::P0, fend_off(), &[blocker], &[]);
    lk::attack(&mut engine, &[ogre]);
    lk::block(&mut engine, &[(blocker, ogre)]);
    lk::through_combat(&mut engine);
    assert!(
        on_battlefield(&engine, lk::P0, gray_ogre()).is_some(),
        "blocking creature deals no damage either"
    );
    assert_eq!(
        lk::damage_on(&engine, blocker),
        2,
        "untargeted attacker still deals damage"
    );
    reach_their_main_phase(&mut engine, lk::P1);
    reach_their_main_phase(&mut engine, lk::P0);
    lk::attack(&mut engine, &[ogre]);
    lk::block(&mut engine, &[(blocker, ogre)]);
    lk::through_combat(&mut engine);
    assert!(
        in_graveyard(&engine, lk::P0, gray_ogre()).is_some(),
        "prevention expired"
    );
}
