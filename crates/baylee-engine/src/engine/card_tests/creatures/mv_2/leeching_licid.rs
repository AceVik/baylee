//! `cards/creatures/mv_2/leeching_licid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn leeching_licid() -> CardIndex {
    card_index("bcb5af89-a304-4de0-86af-b3f4cfabafc4")
}

/// Leeching Licid (`bcb5af89-a304-4de0-86af-b3f4cfabafc4`) — {1}{B} Creature — Licid (Partial):
/// "{B}, {T}: ... deals 1 damage to that player."
#[test]
fn leeching_licid_deals_one_damage_to_target_creatures_controller() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[leeching_licid(), swamp()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, leeching_licid(), 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 19);
}
