#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn war_dance() -> CardIndex {
    card_index("ecfc81df-1271-47ca-9184-49bf5798e195")
}

#[test]
fn war_dance_adds_verse_counter_and_pumps_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[war_dance(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("ogre on bf");
    activate(&mut engine, p0, war_dance(), 1);
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, ogre), (3, 3));
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert_eq!(pt(&engine, ogre), (2, 2));
}

#[test]
fn war_dance_can_decline_the_counter_and_zero_pump_is_legal() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game_with(SEED, &[war_dance()], &[gray_ogre()], &[], &[], |e| {
        lk::yes_no(e, false);
    });
    let ogre = lk::obj(&engine, p1, gray_ogre());
    activate(&mut engine, p0, war_dance(), 1);
    lk::aim(&mut engine, &[ogre], &[]);
    assert!(
        in_graveyard(&engine, p0, war_dance()).is_some(),
        "sacrifice is a cost"
    );
    lk::settle(&mut engine);
    assert_eq!(pt(&engine, ogre), (2, 2));
}
