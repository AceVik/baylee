//! `cards/instants/mv_3/jolt.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, JOLT};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Jolt (3e0c0817-317d-439f-b234-2afe0ab29636) — {2}{U} Instant:
/// "You may tap or untap target artifact, creature, or land. Draw a card at the
/// beginning of the next turn's upkeep."
#[test]
fn jolt_taps_target_permanent_and_draws_a_card_at_next_upkeep() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[JOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();
    let before_hand = engine.state().zones.list(ZoneLocation::Hand(P0)).len();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, JOLT);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );

    // Advance to P1's main phase so the delayed upkeep draw resolves.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        before_hand - 1 + 1
    );
}
