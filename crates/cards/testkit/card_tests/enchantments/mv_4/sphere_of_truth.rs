//! `cards/enchantments/mv_4/sphere_of_truth.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn sphere_of_truth() -> CardIndex {
    card_index("e6325604-cdce-4872-af03-e436631004de")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

/// Prevents 2 of the damage a white source would deal to YOU (2 -> 0, 4 -> 2),
/// without preventing off-color sources or white damage to your opponent.
#[test]
fn sphere_of_truth_prevents_two_damage_from_white_sources_to_you_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[sphere_of_truth(), savannah_lions()],
        &[savannah_lions(), serra_angel(), grizzly_bears()],
        &[],
        &[],
    );
    let my_lions = lk::obj(&e, p0, savannah_lions());
    lk::attack(&mut e, &[my_lions]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p1), 18);

    // P1 attacks with White 2/1 (2 -> 0), White 4/4 (4 -> 2), Green 2/2 (unprevented 2).
    reach_their_main_phase(&mut e, p1);
    let opp_lions = lk::obj(&e, p1, savannah_lions());
    let opp_angel = lk::obj(&e, p1, serra_angel());
    let opp_bear = lk::obj(&e, p1, grizzly_bears());
    lk::attack(&mut e, &[opp_lions, opp_angel, opp_bear]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p0), 16);
}
