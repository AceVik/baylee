//! `cards/enchantments/mv_4/sphere_of_grace.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn sphere_of_grace() -> CardIndex {
    card_index("e45de1f7-ad45-494d-855e-3e11f109909d")
}

fn scathe_zombies() -> CardIndex {
    card_index("e0fefaf0-da20-4d58-8db7-019dba16c780")
}

fn sengir_vampire() -> CardIndex {
    card_index("749141aa-f6c4-4ad8-b146-406e68ae9b0b")
}

/// Prevents 2 of the damage a black source would deal to YOU (2 -> 0, 4 -> 2),
/// without preventing off-color sources or black damage to your opponent.
#[test]
fn sphere_of_grace_prevents_two_damage_from_black_sources_to_you_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[sphere_of_grace(), scathe_zombies()],
        &[scathe_zombies(), sengir_vampire(), grizzly_bears()],
        &[],
        &[],
    );
    let my_zombie = lk::obj(&e, p0, scathe_zombies());
    lk::attack(&mut e, &[my_zombie]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p1), 18);

    // P1 attacks with Black 2/2 (2 -> 0), Black 4/4 (4 -> 2), Green 2/2 (unprevented 2).
    reach_their_main_phase(&mut e, p1);
    let opp_zombie = lk::obj(&e, p1, scathe_zombies());
    let opp_vampire = lk::obj(&e, p1, sengir_vampire());
    let opp_bear = lk::obj(&e, p1, grizzly_bears());
    lk::attack(&mut e, &[opp_zombie, opp_vampire, opp_bear]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p0), 16);
}
