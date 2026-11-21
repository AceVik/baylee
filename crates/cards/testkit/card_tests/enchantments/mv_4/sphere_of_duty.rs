//! `cards/enchantments/mv_4/sphere_of_duty.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn sphere_of_duty() -> CardIndex {
    card_index("c1612eef-5059-46fd-a5fd-c8ebf6779a6e")
}

fn garruk_s_companion() -> CardIndex {
    card_index("32fc8fa7-7e0a-4d4e-85cf-2f98b3fc6ecf")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Prevents 2 of the damage a green source would deal to YOU (1 -> 0, 3 -> 1),
/// while off-color sources or damage dealt to your opponent are not prevented.
#[test]
fn sphere_of_duty_prevents_two_damage_from_green_sources_to_you_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[sphere_of_duty(), garruk_s_companion()],
        &[llanowar_elves(), garruk_s_companion(), savannah_lions()],
        &[],
        &[],
    );
    let my_beast = lk::obj(&e, p0, garruk_s_companion());
    lk::attack(&mut e, &[my_beast]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p1), 17);

    // P1 attacks with Green 1/1 (1 -> 0), Green 3/2 (3 -> 1), and White 2/1 (unprevented 2).
    reach_their_main_phase(&mut e, p1);
    let opp_elves = lk::obj(&e, p1, llanowar_elves());
    let opp_beast = lk::obj(&e, p1, garruk_s_companion());
    let opp_lions = lk::obj(&e, p1, savannah_lions());
    lk::attack(&mut e, &[opp_elves, opp_beast, opp_lions]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p0), 17);
}
