//! `cards/creatures/mv_2/goblin_tinkerer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GOBLIN_TINKERER;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Goblin Tinkerer (fbabaa07-236a-45ef-991a-017a0477f516) — {1}{R} 1/2:
/// "{R}, {T}: Destroy target artifact. That artifact deals damage equal to its
/// mana value to this creature."
#[test]
fn goblin_tinkerer_destroys_artifact_and_takes_mana_value_damage() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[GOBLIN_TINKERER, mountain()])
        .battlefield(1, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let tinkerer = on_battlefield(&engine, P0, GOBLIN_TINKERER).unwrap();
    let ring = on_battlefield(&engine, P1, sol_ring()).unwrap();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, GOBLIN_TINKERER, 0);
    aim_at(&mut engine, P0, ring);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, sol_ring()).is_none());
    assert_eq!(engine.state().object(tinkerer).unwrap().damage, 1);
}
