//! `cards/creatures/mv_4/chaotic_goo.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn chaotic_goo() -> CardIndex {
    card_index("d1005625-0a78-4f8e-bf51-80458fc0e9d6")
}

/// p0 casts Chaotic Goo with four Mountains in their first main phase, and
/// the game is handed back with the Goo on the battlefield.
fn a_goo_on_the_table(seed: u64) -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(seed, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[chaotic_goo()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, chaotic_goo());
    pass_until(&mut engine, stack_is_empty);
    let goo = on_battlefield(&engine, P0, chaotic_goo()).unwrap();
    (engine, goo)
}

/// Walks to p0's next upkeep and stops on the "you may flip a coin" question.
#[track_caller]
fn to_my_upkeep_question(engine: &mut Engine<RegistryLookup>) {
    reach_their_main_phase(engine, P1);
    pass_until(engine, |e| {
        e.state().turn.active == P0 && matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, P0, "the Goo's controller is the one who may flip");
}

/// Chaotic Goo (d1005625-0a78-4f8e-bf51-80458fc0e9d6) — {2}{R}{R} Creature — Ooze:
/// "This creature enters with three +1/+1 counters on it. At the beginning of your upkeep, you may flip a coin. If you win the flip, put a +1/+1 counter on this creature. If you lose the flip, remove a +1/+1 counter from this creature."
#[test]
fn chaotic_goo_enters_with_three_counters_and_nothing_happens_on_the_opponents_upkeep() {
    let (mut engine, goo) = a_goo_on_the_table(SEED);
    assert_eq!(pt(&engine, goo), (3, 3));

    // `pass_until` answers a "may" with yes, so a flip at p1's upkeep would
    // show up as a changed size.
    reach_their_main_phase(&mut engine, P1);

    assert_eq!(
        pt(&engine, goo),
        (3, 3),
        "\"your upkeep\" is p0's only: p1's upkeep asks nothing"
    );
}

#[test]
fn chaotic_goo_declining_the_flip_changes_nothing() {
    let (mut engine, goo) = a_goo_on_the_table(SEED);

    to_my_upkeep_question(&mut engine);
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, goo), (3, 3), "a \"may\" declined is not a flip");
}

#[test]
fn chaotic_goo_grows_on_a_won_flip_and_shrinks_on_a_lost_one() {
    // The coin is the game's seeded one, so a handful of seeds shows both
    // faces; a single flip can only ever show one.
    let mut grew = false;
    let mut shrank = false;
    for seed in 1..=24_u64 {
        let (mut engine, goo) = a_goo_on_the_table(seed);
        to_my_upkeep_question(&mut engine);
        engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
        pass_until(&mut engine, stack_is_empty);

        match pt(&engine, goo) {
            (4, 4) => grew = true,
            (2, 2) => shrank = true,
            other => panic!("seed {seed}: one counter either way, got {other:?}"),
        }
    }
    assert!(grew, "a won flip puts a +1/+1 counter on the Goo");
    assert!(shrank, "a lost flip removes one");
}
