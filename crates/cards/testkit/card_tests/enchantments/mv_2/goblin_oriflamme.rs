//! Oracle checklist: +1/+0 applies only to attacking creatures you control; your nonattacker and an opponent's attacker stay unchanged; bonus expires after combat.
#[allow(clippy::wildcard_imports)]
use super::*;
fn goblin_oriflamme() -> CardIndex {
    card_index("836bd011-2da5-443a-a814-19a664b98a1a")
}
#[test]
fn oriflamme_buffs_only_its_controllers_attackers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[goblin_oriflamme(), wild_elephant(), gray_ogre()])
        .battlefield(1, &[wild_elephant()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let attacker = on_battlefield(&e, p0, wild_elephant()).unwrap();
    let nonattacker = on_battlefield(&e, p0, gray_ogre()).unwrap();
    let theirs = on_battlefield(&e, p1, wild_elephant()).unwrap();
    assert_eq!(pt(&e, attacker), (3, 3));
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers { attackers, .. } = e.pending().clone() else {
        unreachable!("pass_until stopped at the attacker choice")
    };
    assert!(attackers.contains(&attacker), "the creature may attack");
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(attacker, baylee_core::ids::Defender::Player(p1))],
        },
    )
    .expect("declare the offered attacker against the opponent");
    assert_eq!(pt(&e, attacker), (4, 3));
    assert_eq!(pt(&e, nonattacker), (2, 2));
    assert_eq!(pt(&e, theirs), (3, 3));

    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == p1),
    );
    e.apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("the opponent leaves the attacker unblocked");
    pass_until(&mut e, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(pt(&e, attacker), (3, 3), "the bonus ends after combat");
    reach_their_main_phase(&mut e, p1);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    let Pending::ChooseAttackers { attackers, .. } = e.pending().clone() else {
        unreachable!("pass_until stopped at the opponent's attacker choice")
    };
    assert!(attackers.contains(&theirs));
    e.apply(
        p1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(theirs, baylee_core::ids::Defender::Player(p0))],
        },
    )
    .expect("the opponent's creature is able to attack");
    assert_eq!(
        pt(&e, theirs),
        (3, 3),
        "the enchantment does not buff an opponent's attacker"
    );
}
