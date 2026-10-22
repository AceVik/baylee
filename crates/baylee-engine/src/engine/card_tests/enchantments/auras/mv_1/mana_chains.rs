//! `cards/enchantments/auras/mv_1/mana_chains.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, MANA_CHAINS};

/// `Mana Chains` ({U}, Enchantment — Aura) grants enchanted creature
/// "Cumulative upkeep `{1}`."
#[test]
fn mana_chains_grants_cumulative_upkeep_to_enchanted_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[GRIZZLY_BEARS, island()])
        .hand(0, &[MANA_CHAINS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, MANA_CHAINS);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, GRIZZLY_BEARS).is_some());
}
