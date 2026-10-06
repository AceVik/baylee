//! `cards/creatures/mv_3/scathe_zombies.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Scathe Zombies — vanilla `{2}{B}` 2/2 Zombie.
#[test]
fn scathe_zombies_is_a_two_two_zombie_for_2b() {
    let p0 = PlayerId::new(0);
    assert_eq!(
        cast_saying_nothing(scathe_zombies(), swamp(), 3),
        Zone::Battlefield
    );
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[scathe_zombies()])
        .start();
    keep_mulligans(&mut engine);
    let id = on_battlefield(&engine, p0, scathe_zombies()).expect("seated");
    assert_eq!(pt(&engine, id), (2, 2));
}
