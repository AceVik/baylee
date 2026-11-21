//! `cards/enchantments/auras/mv_2/psionic_gift.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn psionic_gift() -> CardIndex {
    card_index("842bb10a-f056-4db8-a341-2d20796b99ba")
}

/// Enchanted creature gains "{T}: This creature deals 1 damage to any target"
/// under that creature's controller.
#[test]
fn psionic_gift_grants_one_damage_ping_to_any_target_for_creatures_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[grizzly_bears()],
        &[grizzly_bears(), llanowar_elves()],
        &[psionic_gift(), psionic_gift()],
        &[],
    );
    let my_bear = lk::obj(&e, p0, grizzly_bears());
    let opp_bear = lk::obj(&e, p1, grizzly_bears());
    let opp_elves = lk::obj(&e, p1, llanowar_elves());

    lk::float(&mut e, p0, &[(ManaColor::Blue, 4)]);
    lk::cast_at(&mut e, p0, psionic_gift(), &[my_bear], &[]);
    lk::cast_at(&mut e, p0, psionic_gift(), &[opp_bear], &[]);

    // Only P0's Bear offers the ability to P0.
    lk::use_ability(
        &mut e,
        p0,
        grizzly_bears(),
        baylee_engine::choice::GRANTED_ABILITY,
        &[opp_elves],
        &[],
    );
    assert_eq!(lk::zone_of(&e, opp_elves), Zone::Graveyard);

    // Next turn P0 can target player; on P1's turn, P1 activates on opp_bear.
    reach_their_main_phase(&mut e, p1);
    lk::use_ability(
        &mut e,
        p1,
        grizzly_bears(),
        baylee_engine::choice::granted_ability(1),
        &[],
        &[p0],
    );
    assert_eq!(lk::life(&e, p0), 19);

    reach_their_main_phase(&mut e, p0);
    lk::use_ability(
        &mut e,
        p0,
        grizzly_bears(),
        baylee_engine::choice::GRANTED_ABILITY,
        &[],
        &[p1],
    );
    assert_eq!(lk::life(&e, p1), 19);
}
