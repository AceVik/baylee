//! `cards/enchantments/auras/mv_3/foul_presence.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn foul_presence() -> CardIndex {
    card_index("16f3a09f-e29b-4fb5-95ed-d76ad550c2ad")
}

fn horned_turtle() -> CardIndex {
    card_index("7a4ae76b-6241-4fa8-b239-5ec941a0e7df")
}

/// Enchanted creature gets -1/-1 and "{T}: Target creature gets -1/-1 until end of turn."
#[test]
fn foul_presence_gives_minus_one_minus_one_and_tap_shrink_ability() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[horned_turtle()],
        &[grizzly_bears(), llanowar_elves()],
        &[foul_presence()],
        &[],
    );
    let turtle = lk::obj(&e, p0, horned_turtle());
    let opp_bear = lk::obj(&e, p1, grizzly_bears());
    let opp_elves = lk::obj(&e, p1, llanowar_elves());

    lk::float(&mut e, p0, &[(ManaColor::Black, 3)]);
    lk::cast_at(&mut e, p0, foul_presence(), &[turtle], &[]);
    assert_eq!(pt(&e, turtle), (0, 3));

    lk::use_ability(
        &mut e,
        p0,
        horned_turtle(),
        baylee_engine::choice::GRANTED_ABILITY,
        &[opp_elves],
        &[],
    );
    assert_eq!(lk::zone_of(&e, opp_elves), Zone::Graveyard);

    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    lk::use_ability(
        &mut e,
        p0,
        horned_turtle(),
        baylee_engine::choice::GRANTED_ABILITY,
        &[opp_bear],
        &[],
    );
    assert_eq!(pt(&e, opp_bear), (1, 1));
    reach_their_main_phase(&mut e, p1);
    assert_eq!(pt(&e, opp_bear), (2, 2));
}
