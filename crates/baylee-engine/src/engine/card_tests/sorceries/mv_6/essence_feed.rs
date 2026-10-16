//! `cards/sorceries/mv_6/essence_feed.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "c29b997a-5be6-4d51-a81b-d00677ef401d"
fn essence_feed() -> CardIndex {
    card_index("c29b997a-5be6-4d51-a81b-d00677ef401d")
}

/// Essence Feed: "Target player loses 3 life. You gain 3 life and create three
/// 0/1 colorless Eldrazi Spawn creature tokens."
#[test]
fn essence_feed_drains_three_and_makes_three_spawn() {
    let mut engine = table(956, &[essence_feed()], &[swamp(); 6], &[], &[]);
    let (_, players) = cast_at_player(&mut engine, essence_feed(), them());
    assert!(players.contains(&me()) && players.contains(&them()), "any player: {players:?}");
    assert_eq!(life_of(&engine, them()), 17, "the target loses three");
    assert_eq!(life_of(&engine, me()), 23, "the caster gains three");
    let spawn = my_tokens(&engine);
    assert_eq!(spawn.len(), 3, "three Spawn");
    for token in spawn {
        assert_eq!(pt(&engine, token), (0, 1));
    }
}
