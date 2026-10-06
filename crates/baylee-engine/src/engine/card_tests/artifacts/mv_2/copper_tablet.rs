//! `cards/artifacts/mv_2/copper_tablet.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Copper Tablet: "At the beginning of each player's upkeep, this artifact
/// deals 1 damage to that player." Each player takes 1 at their own upkeep
/// and none at the other's — p0's first upkeep is the very first of the
/// game and still fires.
#[test]
fn copper_tablet_deals_1_to_each_player_at_their_own_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[copper_tablet()])
        .start();
    keep_mulligans(&mut engine);
    assert_eq!(life_of(&engine, p0), 20, "before p0's first upkeep");
    assert_eq!(life_of(&engine, p1), 20, "before anyone's upkeep");

    reach_main_phase(&mut engine, p0);
    assert_eq!(life_of(&engine, p0), 19, "p0 took 1 at their own upkeep");
    assert_eq!(life_of(&engine, p1), 20, "not yet p1's upkeep");

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(life_of(&engine, p1), 19, "p1 took 1 at their own upkeep");
    assert_eq!(life_of(&engine, p0), 19, "p0's life did not move again");
}
