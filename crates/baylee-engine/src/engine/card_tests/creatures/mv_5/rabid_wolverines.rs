use super::*;

fn rabid_wolverines() -> CardIndex {
    card_index("ed076c39-9bfc-424a-8ce8-8572f7c0de73")
}

#[test]
fn rabid_wolverines_gets_plus_1_plus_1_for_each_blocking_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[rabid_wolverines()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wolverines = on_battlefield(&engine, p0, rabid_wolverines()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    assert_eq!(pt(&engine, wolverines), (4, 4));

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(wolverines, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(ogre, wolverines)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, wolverines), (5, 5));
}
