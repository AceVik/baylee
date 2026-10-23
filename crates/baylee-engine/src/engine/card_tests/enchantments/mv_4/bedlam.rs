#[allow(clippy::wildcard_imports)]
use super::*;

fn bedlam() -> CardIndex {
    card_index("c28d4caf-9b7c-40b1-9cbe-275ca9dae06a")
}

#[test]
fn bedlam_prevents_creatures_from_blocking() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[bedlam()])
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, bedlam());
    pass_until(&mut engine, stack_is_empty);
    let attacker = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    let blocker = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, baylee_core::ids::Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(blocker, attacker)],
                },
            )
            .is_err()
    );
}
