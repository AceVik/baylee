//! `cards/creatures/mv_1/dwarven_scorcher.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn dwarven_scorcher() -> CardIndex {
    card_index("a896cedb-f10a-48ac-82e7-6ae1e8b5a439")
}

/// Dwarven Scorcher ({R} 1/1 Dwarf):
/// "Sacrifice this creature: This creature deals 1 damage to target creature
/// unless that creature's controller has this creature deal 2 damage to them."
#[test]
fn dwarven_scorcher_deals_one_to_target_when_its_controller_declines_two_damage() {
    let mut e = lk::game(SEED, &[dwarven_scorcher()], &[benalish_hero()], &[], &[]);
    let hero = lk::obj(&e, lk::P1, benalish_hero());

    activate(&mut e, lk::P0, dwarven_scorcher(), 0);
    lk::aim(&mut e, &[hero], &[]);
    assert!(on_battlefield(&e, lk::P0, dwarven_scorcher()).is_none());
    assert!(in_graveyard(&e, lk::P0, dwarven_scorcher()).is_some());

    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    let Pending::YesNo { player, .. } = e.pending().clone() else {
        unreachable!();
    };
    assert_eq!(player, lk::P1, "target creature's controller decides");
    e.apply(lk::P1, PlayerAction::YesNo(false)).unwrap();
    lk::settle(&mut e);

    assert!(on_battlefield(&e, lk::P1, benalish_hero()).is_none());
    assert_eq!(lk::life(&e, lk::P1), 20);
}

#[test]
fn dwarven_scorcher_deals_two_to_target_controller_when_accepted_sparing_the_creature() {
    let mut e = lk::game(SEED, &[dwarven_scorcher()], &[benalish_hero()], &[], &[]);
    let hero = lk::obj(&e, lk::P1, benalish_hero());

    activate(&mut e, lk::P0, dwarven_scorcher(), 0);
    lk::aim(&mut e, &[hero], &[]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(lk::P1, PlayerAction::YesNo(true)).unwrap();
    lk::settle(&mut e);

    assert!(
        on_battlefield(&e, lk::P1, benalish_hero()).is_some(),
        "accepting 2 player damage prevents the 1 damage to the target creature"
    );
    assert_eq!(lk::damage_on(&e, hero), 0);
    assert_eq!(lk::life(&e, lk::P1), 18);
}
