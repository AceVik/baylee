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

#[test]
fn bedlam_also_prevents_its_controllers_creatures_from_blocking() {
    use crate::engine::card_tests::legends_kit as lk;
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(
        SEED,
        &[bedlam(), grizzly_bears()],
        &[grizzly_bears()],
        &[],
        &[],
    );
    reach_their_main_phase(&mut engine, p1);
    let attacker = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let blocker = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let offer = lk::attack(&mut engine, &[attacker]);
    assert!(!lk::may_block(&offer, blocker, attacker));
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(blocker, attacker)]
                }
            )
            .is_err()
    );
    lk::block(&mut engine, &[]);
}
