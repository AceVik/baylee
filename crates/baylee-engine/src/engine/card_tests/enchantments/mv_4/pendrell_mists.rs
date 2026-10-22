//! `cards/enchantments/mv_4/pendrell_mists.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, PENDRELL_MISTS};

/// `Pendrell Mists` ({3}{U}, Enchantment) gives all creatures "At the beginning
/// of your upkeep, sacrifice this creature unless you pay `{1}`."
#[test]
fn pendrell_mists_forces_creature_sacrifice_unless_paid_at_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[PENDRELL_MISTS, GRIZZLY_BEARS, island()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, GRIZZLY_BEARS).is_some());
}
