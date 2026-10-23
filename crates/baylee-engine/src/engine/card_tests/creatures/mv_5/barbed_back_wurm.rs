//! `cards/creatures/mv_5/barbed_back_wurm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn barbed_back_wurm() -> CardIndex {
    card_index("d60cbdf0-f20f-4306-be6b-eed90d2a2f77")
}

#[test]
fn barbed_back_wurm_shrinks_green_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[barbed_back_wurm(), swamp()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p0, barbed_back_wurm()).unwrap();
    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(wurm, Defender::Player(p1))],
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
                blockers: vec![(bear, wurm)],
            },
        )
        .unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, barbed_back_wurm(), 0);
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bear), (1, 1));
}

#[test]
fn barbed_back_wurm_cannot_target_a_nongreen_blocker_or_a_green_creature_that_is_not_blocking() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[barbed_back_wurm(), swamp()])
        .battlefield(1, &[HILL_GIANT, GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wurm = on_battlefield(&engine, p0, barbed_back_wurm()).unwrap();
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(wurm, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    // The red Hill Giant blocks; the green Grizzly Bears stays back.
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, wurm)],
            },
        )
        .unwrap();
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("priority, got {:?}", engine.pending())
    };
    assert!(!legal.abilities.contains(&(wurm, 0)));
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: wurm,
                    ability_index: 0,
                },
            )
            .is_err(),
        "the only blocker is not green, and the green creature is not blocking"
    );
}
