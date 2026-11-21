//! `cards/enchantments/mv_4/sphere_of_reason.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn sphere_of_reason() -> CardIndex {
    card_index("de85c961-622d-4d70-8cd7-f722c1ac5ed1")
}

fn fugitive_wizard() -> CardIndex {
    card_index("381fa2a9-69fc-4558-a0ac-fb99c6f8d77f")
}

fn air_elemental() -> CardIndex {
    card_index("7744bae4-a8b7-44a5-9b4c-0048ad4cc448")
}

/// Prevents 2 of the damage a blue source would deal to YOU (1 -> 0, 4 -> 2),
/// without preventing off-color sources or blue damage to your opponent.
#[test]
fn sphere_of_reason_prevents_two_damage_from_blue_sources_to_you_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[sphere_of_reason(), air_elemental()],
        &[fugitive_wizard(), air_elemental(), grizzly_bears()],
        &[],
        &[],
    );
    let my_elem = lk::obj(&e, p0, air_elemental());
    lk::attack(&mut e, &[my_elem]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p1), 16);

    // P1 attacks with Blue 1/1 (1 -> 0), Blue 4/4 (4 -> 2), Green 2/2 (unprevented 2).
    reach_their_main_phase(&mut e, p1);
    let opp_wiz = lk::obj(&e, p1, fugitive_wizard());
    let opp_elem = lk::obj(&e, p1, air_elemental());
    let opp_bear = lk::obj(&e, p1, grizzly_bears());
    lk::attack(&mut e, &[opp_wiz, opp_elem, opp_bear]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p0), 16);
}
