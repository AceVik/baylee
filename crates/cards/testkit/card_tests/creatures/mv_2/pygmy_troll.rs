use super::*;

fn pygmy_troll() -> CardIndex {
    card_index("6f18f41e-df8d-4b59-87cc-ea167823fbda")
}

#[test]
fn pygmy_troll_gets_plus_one_plus_one_when_blocked_and_can_regenerate() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[pygmy_troll(), forest()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let troll = on_battlefield(&engine, p0, pygmy_troll()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, pygmy_troll(), 1);
    pass_until(&mut engine, stack_is_empty);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(troll, Defender::Player(p1))],
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
                blockers: vec![(ogre, troll)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, troll), (2, 2));
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        on_battlefield(&engine, p0, pygmy_troll()).is_some(),
        "regeneration saves the troll from lethal combat damage"
    );
    assert!(is_tapped(&engine, troll));
    assert_eq!(engine.state().object(troll).unwrap().damage, 0);
    assert!(in_graveyard(&engine, p1, gray_ogre()).is_some());
}
