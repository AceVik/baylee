//! `cards/creatures/mv_3/akki_coalflinger.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn akki_coalflinger() -> CardIndex {
    card_index("96de29a1-3178-4c8b-a67b-ab0262442cb6")
}

#[test]
fn akki_coalflingers_printed_first_strike_wins_against_an_even_bear() {
    let mut e = lk::game(SEED, &[akki_coalflinger()], &[grizzly_bears()], &[], &[]);
    let attacker = lk::obj(&e, lk::P0, akki_coalflinger());
    let blocker = lk::obj(&e, lk::P1, grizzly_bears());
    lk::attack(&mut e, &[attacker]);
    lk::block(&mut e, &[(blocker, attacker)]);
    lk::through_combat(&mut e);
    assert!(on_battlefield(&e, lk::P0, akki_coalflinger()).is_some());
    assert!(in_graveyard(&e, lk::P1, grizzly_bears()).is_some());
}

#[test]
fn akki_coalflinger_grants_first_strike_only_to_attackers_until_cleanup() {
    let mut e = lk::game(
        SEED,
        &[akki_coalflinger(), index::RAGING_GOBLIN, grizzly_bears()],
        &[],
        &[],
        &[],
    );
    let attacker = lk::obj(&e, lk::P0, index::RAGING_GOBLIN);
    let bystander = lk::obj(&e, lk::P0, grizzly_bears());
    lk::attack(&mut e, &[attacker]);
    lk::block(&mut e, &[]);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    activate(&mut e, lk::P0, akki_coalflinger(), 0);
    lk::settle(&mut e);
    assert!(lk::tapped(&e, lk::obj(&e, lk::P0, akki_coalflinger())));
    assert!(keywords(&e, attacker).contains(KeywordSet::FIRST_STRIKE));
    assert!(!keywords(&e, bystander).contains(KeywordSet::FIRST_STRIKE));
    lk::through_combat(&mut e);
    assert!(keywords(&e, attacker).contains(KeywordSet::FIRST_STRIKE));
    pass_until(&mut e, |e| e.state().turn.active != lk::P0);
    assert!(!keywords(&e, attacker).contains(KeywordSet::FIRST_STRIKE));
}
