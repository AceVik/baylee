//! `cards/creatures/mv_2/mogg_war_marshal.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn marshal() -> CardIndex {
    card_index("7b8f4879-578e-40e4-863a-41d0c32c6bdd")
}

#[test]
fn mogg_war_marshal_makes_one_goblin_on_entry_and_another_on_death() {
    let mut e = lk::game(SEED, &[], &[], &[marshal()], &[]);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 2)]);
    cast_with_floating(&mut e, lk::P0, marshal());
    lk::settle(&mut e);
    assert_eq!(tokens_of(&e, lk::P0).len(), 1);
    assert_eq!(pt(&e, tokens_of(&e, lk::P0)[0]), (1, 1));

    let source = lk::obj(&e, lk::P0, marshal());
    kill(&mut e, source);
    assert!(in_graveyard(&e, lk::P0, marshal()).is_some());
    assert_eq!(tokens_of(&e, lk::P0).len(), 2);
}

#[test]
fn mogg_war_marshal_echo_paid_survives_without_dying_trigger() {
    let mut e = lk::game(SEED, &[], &[], &[marshal()], &[]);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 2)]);
    cast_with_floating(&mut e, lk::P0, marshal());
    lk::settle(&mut e);
    reach_their_main_phase(&mut e, lk::P1);
    pass_until(&mut e, |e| {
        e.state().turn.active == lk::P0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 2)]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, true);
    reach_main_phase(&mut e, lk::P0);
    assert!(on_battlefield(&e, lk::P0, marshal()).is_some());
    assert_eq!(tokens_of(&e, lk::P0).len(), 1);
    assert_eq!(lk::pool_total(&e, lk::P0), 0);
}

#[test]
fn mogg_war_marshal_unpaid_echo_dies_and_makes_another_goblin() {
    let mut e = lk::game(SEED, &[], &[], &[marshal()], &[]);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 2)]);
    cast_with_floating(&mut e, lk::P0, marshal());
    lk::settle(&mut e);
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    assert!(in_graveyard(&e, lk::P0, marshal()).is_some());
    assert_eq!(tokens_of(&e, lk::P0).len(), 2);
}
