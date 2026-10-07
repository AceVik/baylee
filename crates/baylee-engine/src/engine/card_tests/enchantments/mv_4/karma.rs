//! `cards/enchantments/mv_4/karma.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Karma: "At the beginning of each player's upkeep, this enchantment deals
/// damage to that player equal to the number of Swamps they control." p0
/// controls none and takes none; p1 controls three and takes three, at
/// their own upkeep.
#[test]
fn karma_deals_upkeep_damage_equal_to_swamps_controlled() {
    let (_p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let karma = karma();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[karma])
        .battlefield(1, &[swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    assert_eq!(engine.state().players[0].life, 20, "p0 controls no Swamps");
    assert_eq!(
        engine.state().players[1].life,
        17,
        "p1's three Swamps hit them at their own upkeep"
    );
}
