#[allow(clippy::wildcard_imports)]
use super::*;

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
}
