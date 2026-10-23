use super::*;

fn reckless_ogre() -> CardIndex {
    card_index("fe455177-d9e5-4893-ad87-ebbba5e2fa45")
}

#[test]
fn reckless_ogre_gets_plus_3_plus_0_when_attacking_alone() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[reckless_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, reckless_ogre()).unwrap();
    assert_eq!(pt(&engine, ogre), (3, 2));

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogre, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, ogre), (6, 2));
}
