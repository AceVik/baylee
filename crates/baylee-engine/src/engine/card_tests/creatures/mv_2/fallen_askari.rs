//! `cards/creatures/mv_2/fallen_askari.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn fallen_askari() -> CardIndex {
    card_index("4956008e-d912-48ba-9b82-176258281060")
}

/// "This creature can't block": p1's Bears attacks into an Askari and a Grizzly Bears of p0's.
/// The declare-blockers question comes to p0, with the Bears on the menu and the Askari not, and
/// naming the Askari as a blocker is refused; the Bears may block.
#[test]
fn fallen_askari_cannot_block() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[fallen_askari(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let askari = on_battlefield(&engine, p0, fallen_askari()).unwrap();
    let my_bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let opp_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(opp_bear, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, p0, "the defending player is asked");
    assert!(
        blockers.iter().any(|option| option.blocker == my_bear),
        "the Bears may block"
    );
    assert!(
        blockers.iter().all(|option| option.blocker != askari),
        "the Askari is not offered as a blocker"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(askari, opp_bear)],
                },
            )
            .is_err(),
        "naming the Askari as a blocker is refused"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(my_bear, opp_bear)],
            },
        )
        .expect("the Bears may block");
}

/// Flanking: a Grizzly Bears (no flanking) that blocks the Askari gets -1/-1
/// until end of turn, so the 1/1 Bears dies to the Askari's 2 damage and the
/// Askari survives with 1 damage.
#[test]
fn fallen_askari_flanking_shrinks_the_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[fallen_askari()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let askari = on_battlefield(&engine, p0, fallen_askari()).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    assert!(keywords(&engine, askari).contains(KeywordSet::FLANKING));
    assert_eq!(pt(&engine, askari), (2, 2));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(askari, Defender::Player(p1))],
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
                blockers: vec![(bear, askari)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bear), (1, 1), "-1/-1 from flanking");

    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd
    });
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert_eq!(
        engine.state().object(askari).unwrap().damage,
        1,
        "the shrunken Bears dealt 1"
    );
    assert!(on_battlefield(&engine, p0, fallen_askari()).is_some());
}
