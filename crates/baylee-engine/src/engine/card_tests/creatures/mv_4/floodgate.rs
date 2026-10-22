//! `cards/creatures/mv_4/floodgate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{FLOODGATE, GRIZZLY_BEARS, SERRA_ANGEL, UNSUMMON};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Floodgate (f5a85674-fd09-42d8-bba7-5656be25fcea) — {3}{U} 0/5:
/// "Defender. This creature can't attack unless defending player controls an
/// Island. When you control no Islands, sacrifice this creature. When this
/// creature leaves the battlefield, it deals damage equal to half the number
/// of Islands you control, rounded down, to each nonblue creature without
/// flying."
#[test]
fn floodgate_deals_half_island_count_damage_to_nonblue_nonflying_creatures_when_leaving_battlefield()
 {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[FLOODGATE, island(), island(), island(), island()])
        .battlefield(1, &[island(), GRIZZLY_BEARS, SERRA_ANGEL])
        .hand(0, &[UNSUMMON])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let gate = on_battlefield(&engine, P0, FLOODGATE).unwrap();
    let angel = on_battlefield(&engine, P1, SERRA_ANGEL).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, UNSUMMON);
    aim_at(&mut engine, P0, gate);
    pass_until(&mut engine, stack_is_empty);

    // 4 Islands / 2 = 2 damage to nonblue creatures without flying:
    // Grizzly Bears (2/2) dies; Serra Angel (flying) takes 0 damage.
    assert!(on_battlefield(&engine, P0, FLOODGATE).is_none());
    assert!(on_battlefield(&engine, P1, GRIZZLY_BEARS).is_none());
    assert_eq!(engine.state().object(angel).unwrap().damage, 0);
}
