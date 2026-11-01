//! `cards/creatures/mv_4/tradewind_rider.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn tradewind_rider() -> CardIndex {
    card_index("d0314da0-da24-4f6e-b1f6-0d50617bcaaa")
}

/// `Tradewind Rider` ({3}{U}, 1/4 Creature — Spirit, `Coverage::Implemented`):
/// `Flying`
/// `{T}, Tap two untapped creatures you control: Return target permanent to its
/// owner's hand.`
#[test]
fn tradewind_rider_taps_itself_and_two_creatures_to_bounce_target_permanent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[tradewind_rider(), GRIZZLY_BEARS, GRIZZLY_BEARS])
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let rider = on_battlefield(&engine, p0, tradewind_rider()).unwrap();
    assert!(
        engine
            .state()
            .object(rider)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING)
    );
    let bears: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .unwrap()
                .card
                .is_some_and(|c| c.index == GRIZZLY_BEARS)
        })
        .collect();
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    activate(&mut engine, p0, tradewind_rider(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![giant]
                }
            )
            .is_err(),
        "only creatures you control can pay the tapping cost"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears[0]],
            },
        )
        .unwrap();
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![bears[0]]
                }
            )
            .is_err(),
        "the same tapped creature cannot pay twice"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears[1]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, HILL_GIANT).is_none());
    assert!(in_hand(&engine, p1, HILL_GIANT).is_some());
    assert!(is_tapped(&engine, rider));
    assert!(bears.into_iter().all(|id| is_tapped(&engine, id)));
}
