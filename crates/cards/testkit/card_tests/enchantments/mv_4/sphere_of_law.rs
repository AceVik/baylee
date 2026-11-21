//! `cards/enchantments/mv_4/sphere_of_law.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn sphere_of_law() -> CardIndex {
    card_index("a8da7c57-c60f-42b1-b611-64a0a43295f2")
}

/// Prevents 2 of the damage a red source would deal to YOU, without preventing
/// off-color damage or red damage dealt to your opponent.
#[test]
fn sphere_of_law_prevents_two_damage_from_red_sources_to_you_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[sphere_of_law()],
        &[grizzly_bears()],
        &[lightning_bolt()],
        &[lightning_bolt()],
    );

    // P0's own Bolt still deals full 3 to P1.
    lk::float(&mut e, p0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, p0, lightning_bolt(), &[], &[p1]);
    assert_eq!(lk::life(&e, p1), 17);

    // P1's Bolt to P0 deals 3 - 2 = 1; P1's Green Bear deals full 2 (20 -> 19 -> 17).
    reach_their_main_phase(&mut e, p1);
    lk::float(&mut e, p1, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, p1, lightning_bolt(), &[], &[p0]);
    assert_eq!(lk::life(&e, p0), 19);

    let opp_bear = lk::obj(&e, p1, grizzly_bears());
    lk::attack(&mut e, &[opp_bear]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p0), 17);
}
