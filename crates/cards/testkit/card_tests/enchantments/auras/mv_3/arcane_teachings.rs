//! `cards/enchantments/auras/mv_3/arcane_teachings.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn arcane_teachings() -> CardIndex {
    card_index("0b633017-bfdf-47cd-a60f-c93816b47e9c")
}

/// Enchanted creature gets +2/+2 and "{T}: This creature deals 1 damage to any target."
#[test]
fn arcane_teachings_gives_plus_two_plus_two_and_ping_ability() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[grizzly_bears(), llanowar_elves()],
        &[llanowar_elves()],
        &[arcane_teachings()],
        &[],
    );
    let bear = lk::obj(&e, p0, grizzly_bears());
    let my_elves = lk::obj(&e, p0, llanowar_elves());
    let opp_elves = lk::obj(&e, p1, llanowar_elves());

    lk::float(&mut e, p0, &[(ManaColor::Red, 3)]);
    lk::cast_at(&mut e, p0, arcane_teachings(), &[bear], &[]);
    assert_eq!(pt(&e, bear), (4, 4));
    assert_eq!(pt(&e, my_elves), (1, 1));
    assert!(!lk::offered(
        &e,
        llanowar_elves(),
        p0,
        baylee_engine::choice::GRANTED_ABILITY
    ));

    lk::use_ability(
        &mut e,
        p0,
        grizzly_bears(),
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
        grizzly_bears(),
        baylee_engine::choice::GRANTED_ABILITY,
        &[],
        &[p1],
    );
    assert_eq!(lk::life(&e, p1), 19);
}
