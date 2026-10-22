//! `cards/sorceries/mv_5/illicit_auction.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{HILL_GIANT, ILLICIT_AUCTION};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Illicit Auction (a624b049-8364-480f-a95c-c82a35ca7eb2) — {3}{R}{R} Sorcery:
/// "Each player may bid life for control of target creature..."
#[test]
fn illicit_auction_gains_control_of_target_creature() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[ILLICIT_AUCTION])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P1, HILL_GIANT).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, ILLICIT_AUCTION);
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(giant).unwrap().controller, P0);
}
