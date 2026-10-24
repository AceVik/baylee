//! `cards/creatures/mv_3/metathran_elite.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn metathran_elite() -> CardIndex {
    card_index("c8a1b3dd-bc36-4d97-8f93-1151446f04e2")
}

/// Metathran Elite (c8a1b3dd-bc36-4d97-8f93-1151446f04e2) — 2/3 with "This
/// creature can't be blocked as long as it's enchanted."
///
/// Unenchanted, an opposing creature may block it; once enchanted by an Aura,
/// it gains unblockable and cannot be blocked.
#[test]
fn metathran_elite_is_blockable_until_enchanted_and_unblockable_while_enchanted() {
    let mut e_plain = lk::game(SEED, &[metathran_elite()], &[llanowar_elves()], &[], &[]);
    let elite_plain = lk::obj(&e_plain, lk::P0, metathran_elite());
    let elf_plain = lk::obj(&e_plain, lk::P1, llanowar_elves());
    assert_eq!(pt(&e_plain, elite_plain), (2, 3));
    assert!(!keywords(&e_plain, elite_plain).contains(KeywordSet::UNBLOCKABLE));

    let offer_plain = lk::attack(&mut e_plain, &[elite_plain]);
    assert!(
        lk::may_block(&offer_plain, elf_plain, elite_plain),
        "unenchanted Metathran Elite can be blocked"
    );

    let mut e_enc = lk::game(
        SEED,
        &[metathran_elite()],
        &[llanowar_elves()],
        &[rancor()],
        &[],
    );
    let elite = lk::obj(&e_enc, lk::P0, metathran_elite());
    let their_elf = lk::obj(&e_enc, lk::P1, llanowar_elves());
    lk::float(&mut e_enc, lk::P0, &[(ManaColor::Green, 1)]);
    cast_with_floating(&mut e_enc, lk::P0, rancor());
    e_enc
        .apply(
            lk::P0,
            PlayerAction::ChooseObjects {
                objects: vec![elite],
            },
        )
        .unwrap();
    lk::settle(&mut e_enc);

    assert!(
        keywords(&e_enc, elite).contains(KeywordSet::UNBLOCKABLE),
        "enchanted Metathran Elite cannot be blocked"
    );
    let offer_enc = lk::attack(&mut e_enc, &[elite]);
    assert!(!lk::may_block(&offer_enc, their_elf, elite));
    assert!(
        e_enc
            .apply(
                lk::P1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(their_elf, elite)],
                },
            )
            .is_err(),
        "declaring a blocker against enchanted Metathran Elite is rejected"
    );
}
