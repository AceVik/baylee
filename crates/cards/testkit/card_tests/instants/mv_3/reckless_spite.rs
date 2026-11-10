//! `cards/instants/mv_3/reckless_spite.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{BOG_IMP, GRIZZLY_BEARS, HILL_GIANT};

fn reckless_spite() -> CardIndex {
    card_index("a684df3a-5441-4daa-86d1-c47a91b35e6a")
}

/// `Reckless Spite` ({1}{B}{B}, Instant, `Coverage::Implemented`):
/// `Destroy two target nonblack creatures. You lose 5 life.`
#[test]
fn reckless_spite_destroys_two_target_nonblack_creatures_and_controller_loses_five_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .battlefield(1, &[GRIZZLY_BEARS, HILL_GIANT, BOG_IMP])
        .hand(0, &[reckless_spite()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    let imp = on_battlefield(&engine, p1, BOG_IMP).unwrap();

    cast_from_hand(&mut engine, p0, reckless_spite());
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![bears],
                    players: vec![]
                }
            )
            .is_err(),
        "exactly two target creatures are required"
    );
    // Black creature cannot be targeted.
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![bears, imp],
                    players: vec![],
                },
            )
            .is_err()
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears, giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, GRIZZLY_BEARS).is_none());
    assert!(on_battlefield(&engine, p1, HILL_GIANT).is_none());
    assert!(on_battlefield(&engine, p1, BOG_IMP).is_some());
    assert_eq!(engine.state().players[0].life, 15);
}
