//! `cards/creatures/mv_2/ardent_soldier.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn ardent_soldier() -> CardIndex {
    card_index("a56d301b-8f4c-4f12-8ee0-34c6b66faee0")
}

/// Ardent Soldier ({1}{W} 1/2 Human Soldier):
/// "Kicker {2}; Vigilance; If this creature was kicked, it enters with a +1/+1
/// counter on it."
#[test]
fn ardent_soldier_kicker_adds_counter_and_vigilance_attacks_untapped() {
    let mut e_base = lk::game(SEED, &[], &[], &[ardent_soldier()], &[]);
    lk::float(&mut e_base, lk::P0, &[(ManaColor::White, 4)]);
    cast_with_floating(&mut e_base, lk::P0, ardent_soldier());
    lk::yes_no(&mut e_base, false);
    lk::settle(&mut e_base);
    let base_id = lk::obj(&e_base, lk::P0, ardent_soldier());
    assert_eq!(pt(&e_base, base_id), (1, 2));

    let mut e = lk::game(SEED, &[], &[], &[ardent_soldier()], &[]);
    lk::float(&mut e, lk::P0, &[(ManaColor::White, 4)]);
    cast_with_floating(&mut e, lk::P0, ardent_soldier());
    lk::yes_no(&mut e, true);
    lk::settle(&mut e);
    let soldier = lk::obj(&e, lk::P0, ardent_soldier());
    assert_eq!(counters_on(&e, soldier, CounterKind::P1P1), 1);
    assert_eq!(pt(&e, soldier), (2, 3));

    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    let _ = lk::attack(&mut e, &[soldier]);
    assert!(!lk::tapped(&e, soldier));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 18);
}
