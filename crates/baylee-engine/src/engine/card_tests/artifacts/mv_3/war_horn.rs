//! Oracle checklist: each attacking creature you control gets +1/+0; opposing attackers and idle creatures receive no bonus; the bonus ends when the creature stops attacking.
#[allow(clippy::wildcard_imports)]
use super::*;

fn war_horn() -> CardIndex {
    card_index("99bc739e-c745-47d8-8401-2f34b6becd85")
}

#[test]
fn horn_pumps_only_the_own_creature_declared_as_an_attacker() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[war_horn(), llanowar_elves(), fangren_hunter()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let attacker = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    let idle = on_battlefield(&e, p0, fangren_hunter()).unwrap();
    let theirs = on_battlefield(&e, p1, llanowar_elves()).unwrap();
    assert_eq!(pt(&e, attacker), (1, 1), "before it attacks");
    assert_eq!(pt(&e, idle), (4, 4));

    // Let the opponent attack first: their attacker still is not controlled by
    // the Horn's controller, even while it is attacking.
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    e.apply(
        p1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(theirs, Defender::Player(p0))],
        },
    )
    .unwrap();
    assert_eq!(
        pt(&e, theirs),
        (1, 1),
        "an opposing attacking creature gets no bonus"
    );
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);

    pass_until(&mut e, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers { attackers, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert!(attackers.contains(&attacker));
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(attacker, Defender::Player(p1))],
        },
    )
    .unwrap();

    assert_eq!(
        pt(&e, attacker),
        (2, 1),
        "the bonus applies once declared attacking"
    );
    assert_eq!(
        pt(&e, idle),
        (4, 4),
        "a tapped or untapped nonattacker is not enough"
    );
    assert_eq!(
        pt(&e, theirs),
        (1, 1),
        "an opposing attacker is outside your control"
    );
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == p1),
    );
    let Pending::ChooseBlockers { player, .. } = e.pending().clone() else {
        panic!("blockers are asked");
    };
    e.apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        e.state().players[1].life,
        18,
        "combat damage uses the +1 power"
    );
    assert_eq!(
        pt(&e, attacker),
        (1, 1),
        "the attacker returns to base P/T in second main"
    );
}
