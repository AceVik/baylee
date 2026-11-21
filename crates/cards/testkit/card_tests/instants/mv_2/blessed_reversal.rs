//! `cards/instants/mv_2/blessed_reversal.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "70e0b676-61a2-4dcf-8f61-d9281467ed43"
fn blessed_reversal() -> CardIndex {
    card_index("70e0b676-61a2-4dcf-8f61-d9281467ed43")
}

/// Blessed Reversal ({1}{W}): gains 3 life for each creature attacking you,
/// ignoring non-attacking creatures.
#[test]
fn blessed_reversal_gains_three_life_per_creature_attacking_you() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[blessed_reversal()])
        .battlefield(
            1,
            &[
                index::GRIZZLY_BEARS,
                index::SAVANNAH_LIONS,
                index::HILL_GIANT,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let lions = lk::obj(&engine, P1, index::SAVANNAH_LIONS);
    let _ = lk::attack(&mut engine, &[bears, lions]);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );

    cast_from_hand(&mut engine, P0, blessed_reversal());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        lk::life(&engine, P0),
        26,
        "two attacking creatures yield +6 life; stay-at-home Hill Giant yields 0"
    );
    assert!(in_graveyard(&engine, P0, blessed_reversal()).is_some());
}

/// Creatures attacking an opponent (not attacking you) contribute 0 life.
#[test]
fn blessed_reversal_counts_zero_when_nothing_attacks_you() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, index::SAVANNAH_LIONS],
        &[],
        &[blessed_reversal()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);

    let _ = lk::attack(&mut engine, &[bears, lions]);
    lk::block(&mut engine, &[]);
    lk::float(&mut engine, P0, &[(ManaColor::White, 2)]);
    cast_with_floating(&mut engine, P0, blessed_reversal());
    lk::settle(&mut engine);

    assert_eq!(
        lk::life(&engine, P0),
        20,
        "your own creatures attacking P1 are not attacking you"
    );
}
