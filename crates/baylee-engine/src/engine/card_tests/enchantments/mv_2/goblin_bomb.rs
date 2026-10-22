//! `cards/enchantments/mv_2/goblin_bomb.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GOBLIN_BOMB;

/// `Goblin Bomb` ({1}{R}, Enchantment) flips a coin at upkeep to add or remove
/// a fuse counter, and removes five fuse counters and sacrifices itself to deal
/// 20 damage to target player or planeswalker.
#[test]
fn goblin_bomb_flips_coin_at_upkeep_and_sacrifices_with_five_fuse_counters() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[GOBLIN_BOMB])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    reach_main_phase(&mut engine, p0);

    let bomb = on_battlefield(&engine, p0, GOBLIN_BOMB).unwrap();
    engine
        .state
        .object_mut(bomb)
        .unwrap()
        .counters
        .set(baylee_cards_dsl::counters::FUSE, 5);
    engine.refresh_offer();
    activate(&mut engine, p0, GOBLIN_BOMB, 1);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, GOBLIN_BOMB).is_some());
    assert_eq!(engine.state().players[1].life, 0);
}
