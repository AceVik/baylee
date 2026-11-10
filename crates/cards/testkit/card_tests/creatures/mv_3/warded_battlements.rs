//! Oracle: Defender (This creature can't attack.) / Attacking creatures you control get +1/+0.
#[allow(clippy::wildcard_imports)] // Shared behavioral harness.
use super::*;
use baylee_core::generated::index;

fn subject() -> CardIndex {
    card_index("7616dacf-44e2-4b03-b120-875cc2e50ea5")
}

// Checklist: defender is excluded from attackers; only own attacking creatures
// get +1/+0, not idle own or enemy creatures; bonus disappears after combat.
#[test]
fn only_own_attackers_gain_power_and_wall_cannot_attack() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[subject(), index::GRAY_OGRE, index::HILL_GIANT])
        .battlefield(1, &[index::GRAY_OGRE])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let wall = on_battlefield(&e, p0, subject()).unwrap();
    let attacker = on_battlefield(&e, p0, index::GRAY_OGRE).unwrap();
    let idle = on_battlefield(&e, p0, index::HILL_GIANT).unwrap();
    assert_eq!(pt(&e, attacker), (2, 2));
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = e.pending() else {
        unreachable!()
    };
    assert!(!attackers.contains(&wall));
    assert!(attackers.contains(&attacker));
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(attacker, Defender::Player(p1))],
        },
    )
    .unwrap();
    assert_eq!(pt(&e, attacker), (3, 2));
    assert_eq!(pt(&e, idle), (3, 3));
    assert_eq!(pt(&e, wall), (0, 3));
    assert_eq!(
        pt(&e, on_battlefield(&e, p1, index::GRAY_OGRE).unwrap()),
        (2, 2)
    );
    let life = e.state().players[1].life;
    pass_until(&mut e, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(e.state().players[1].life, life - 3);
    assert_eq!(pt(&e, attacker), (2, 2));
}

// Controller boundary: an opposing attacker receives no bonus while the Wall stays.
#[test]
fn opposing_attacker_keeps_its_printed_power_and_damage() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[subject(), index::HILL_GIANT])
        .battlefield(1, &[index::GRAY_OGRE])
        .start();
    keep_mulligans(&mut e);
    reach_their_main_phase(&mut e, p1);
    let wall = on_battlefield(&e, p0, subject()).unwrap();
    let attacker = on_battlefield(&e, p1, index::GRAY_OGRE).unwrap();
    let idle = on_battlefield(&e, p0, index::HILL_GIANT).unwrap();
    attack_and_collect_blocks(&mut e, attacker, p0);
    assert_eq!(pt(&e, attacker), (2, 2));
    assert_eq!(pt(&e, wall), (0, 3));
    assert_eq!(pt(&e, idle), (3, 3));
    let life = e.state().players[0].life;
    e.apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(e.state().players[0].life, life - 2);
    assert_eq!(on_battlefield(&e, p0, subject()), Some(wall));
}
