//! `cards/creatures/mv_5/hulking_cyclops.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn hulking_cyclops() -> CardIndex {
    card_index("04936918-7ef3-42fa-b0cd-87d18cb69d2c")
}

/// `Hulking Cyclops` ({3}{R}{R}, 5/5 Cyclops, `Coverage::Implemented`) can't
/// block.
#[test]
fn hulking_cyclops_cannot_block() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[hulking_cyclops()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, hulking_cyclops());
    pass_until(&mut engine, stack_is_empty);
    let cyclops = on_battlefield(&engine, p0, hulking_cyclops()).unwrap();
    assert_eq!(pt(&engine, cyclops), (5, 5));

    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let my_bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the question")
    };
    assert!(
        blockers.iter().any(|option| option.blocker == my_bear),
        "the Bears may block"
    );
    assert!(
        blockers.iter().all(|option| option.blocker != cyclops),
        "the Cyclops is not offered as a blocker"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(cyclops, bear)],
                },
            )
            .is_err(),
        "Hulking Cyclops cannot block"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(my_bear, bear)],
            },
        )
        .expect("the Bears may block");
}
