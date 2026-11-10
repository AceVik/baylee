//! `cards/creatures/mv_2/goblin_lookout.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Goblin Lookout (`e4ac0c25-cdfa-49a4-9b2c-b848f50c604d`) — {1}{R} Creature — Goblin 1/2:
/// "{T}, Sacrifice a Goblin: Goblin creatures get +2/+0 until end of turn."
#[test]
fn goblin_lookout_sacrifices_goblin_to_pump_all_goblins() {
    let lookout_card = card_index("e4ac0c25-cdfa-49a4-9b2c-b848f50c604d");
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[lookout_card, index::RAGING_GOBLIN, grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lookout = on_battlefield(&engine, p0, lookout_card).unwrap();
    let fodder = on_battlefield(&engine, p0, index::RAGING_GOBLIN).unwrap();
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();

    activate(&mut engine, p0, lookout_card, 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, index::RAGING_GOBLIN).is_some());
    assert_eq!(pt(&engine, lookout), (3, 2));
    assert_eq!(pt(&engine, bear), (2, 2));
}
