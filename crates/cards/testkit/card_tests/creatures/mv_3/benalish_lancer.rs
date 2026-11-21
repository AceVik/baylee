//! `cards/creatures/mv_3/benalish_lancer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn benalish_lancer() -> CardIndex {
    card_index("b5adbd37-8d98-4c65-b42d-cd9db94e7222")
}

/// Benalish Lancer ({2}{W} 2/2 Human Knight):
/// "Kicker {2}{W}; If this creature was kicked, it enters with two +1/+1
/// counters on it and with first strike."
#[test]
fn benalish_lancer_unkicked_is_two_two_and_kicked_enters_four_four_with_first_strike() {
    let mut e_base = lk::game(SEED, &[], &[], &[benalish_lancer()], &[]);
    lk::float(&mut e_base, lk::P0, &[(ManaColor::White, 6)]);
    cast_with_floating(&mut e_base, lk::P0, benalish_lancer());
    lk::yes_no(&mut e_base, false);
    lk::settle(&mut e_base);
    let base_id = lk::obj(&e_base, lk::P0, benalish_lancer());
    assert_eq!(pt(&e_base, base_id), (2, 2));
    assert!(!keywords(&e_base, base_id).contains(KeywordSet::FIRST_STRIKE));

    // Kicked: 4/4 with first strike kills a 3/3 Hill Giant without taking damage.
    let mut e = lk::game(SEED, &[], &[index::HILL_GIANT], &[benalish_lancer()], &[]);
    let giant = lk::obj(&e, lk::P1, index::HILL_GIANT);
    lk::float(&mut e, lk::P0, &[(ManaColor::White, 6)]);
    cast_with_floating(&mut e, lk::P0, benalish_lancer());
    lk::yes_no(&mut e, true);
    lk::settle(&mut e);
    let lancer = lk::obj(&e, lk::P0, benalish_lancer());
    assert_eq!(counters_on(&e, lancer, CounterKind::P1P1), 2);
    assert_eq!(pt(&e, lancer), (4, 4));
    assert!(keywords(&e, lancer).contains(KeywordSet::FIRST_STRIKE));

    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    let _ = lk::attack(&mut e, &[lancer]);
    lk::block(&mut e, &[(giant, lancer)]);
    lk::through_combat(&mut e);
    assert!(in_graveyard(&e, lk::P1, index::HILL_GIANT).is_some());
    assert_eq!(lk::damage_on(&e, lancer), 0);
}
