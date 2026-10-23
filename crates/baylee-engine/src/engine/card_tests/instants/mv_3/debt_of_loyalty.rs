//! `cards/instants/mv_3/debt_of_loyalty.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{DEBT_OF_LOYALTY, GRIZZLY_BEARS, LIGHTNING_BOLT};

/// `Debt of Loyalty` ({1}{W}{W}, Instant) regenerates target creature.
#[test]
fn debt_of_loyalty_regenerates_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[GRIZZLY_BEARS, plains(), plains(), plains(), mountain()],
        )
        .hand(0, &[DEBT_OF_LOYALTY, LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, DEBT_OF_LOYALTY);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, p0, LIGHTNING_BOLT);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, GRIZZLY_BEARS).is_some());
    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
}
