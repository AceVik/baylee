//! `cards/creatures/mv_4/kezzerdrix.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn kezzerdrix() -> CardIndex {
    card_index("1523671c-0cf4-442c-a5f8-c74ae31016e6")
}

/// Kezzerdrix (`1523671c-0cf4-442c-a5f8-c74ae31016e6`) — {2}{B}{B} Creature — Rabbit Beast:
/// "First strike. At the beginning of your upkeep, if your opponents control no creatures, this creature deals 4 damage to you."
#[test]
fn kezzerdrix_deals_four_damage_at_upkeep_only_when_opponents_control_no_creatures() {
    // When opponent controls no creatures, P0 takes 4 damage at P0's upkeep.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kezzerdrix()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    assert_eq!(engine.state().players[0].life, 16);
    let k = on_battlefield(&engine, P0, kezzerdrix()).unwrap();
    assert!(keywords(&engine, k).contains(KeywordSet::FIRST_STRIKE));
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(engine.state().players[0].life, 16);

    // When opponent controls a creature, Kezzerdrix does not trigger at P0's upkeep.
    let mut engine2 = Duel::new(SEED, forest())
        .battlefield(0, &[kezzerdrix()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, P0);
    assert_eq!(engine2.state().players[0].life, 20);
}
