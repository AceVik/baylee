//! `cards/instants/mv_5/cinder_cloud.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{HILL_GIANT, REVERED_DEAD, SERRA_ANGEL};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cinder_cloud() -> CardIndex {
    card_index("f64ea1a7-0e10-4549-8056-25ebc2ce0175")
}

/// Cinder Cloud (f64ea1a7-0e10-4549-8056-25ebc2ce0175) — {3}{R}{R} Instant:
/// "Destroy target creature. If a white creature dies this way, Cinder Cloud
/// deals damage to that creature's controller equal to the creature's power."
#[test]
fn cinder_cloud_destroys_creature_and_damages_controller_only_if_white() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .battlefield(1, &[SERRA_ANGEL, HILL_GIANT])
        .hand(0, &[cinder_cloud(), cinder_cloud()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let angel = on_battlefield(&engine, P1, SERRA_ANGEL).unwrap();
    let giant = on_battlefield(&engine, P1, HILL_GIANT).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, cinder_cloud());
    aim_at(&mut engine, P0, angel);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, SERRA_ANGEL).is_none());
    assert_eq!(engine.state().players[1].life, 16);

    // Second cast on nonwhite Hill Giant destroys it without dealing damage.
    engine.dev_state_mut(P0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Red, 5);
    engine.refresh_offer();
    cast_with_floating(&mut engine, P0, cinder_cloud());
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, HILL_GIANT).is_none());
    assert_eq!(engine.state().players[1].life, 16);
}

/// "If a white creature dies this way": a white creature that regenerates
/// (Revered Dead, {W}) does not die, so its controller takes no damage.
#[test]
fn cinder_cloud_deals_no_damage_when_the_white_creature_does_not_die() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .battlefield(1, &[REVERED_DEAD, plains()])
        .hand(0, &[cinder_cloud()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let dead = on_battlefield(&engine, P1, REVERED_DEAD).unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, cinder_cloud());
    aim_at(&mut engine, P0, dead);

    // The opponent answers with {W}: regenerate.
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, P1);
    activate(&mut engine, P1, REVERED_DEAD, 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, REVERED_DEAD).is_some());
    assert_eq!(engine.state().players[1].life, 20);
}
