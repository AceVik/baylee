//! `cards/creatures/mv_6/aboroth.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn aboroth() -> CardIndex {
    card_index("28f70f86-19a9-4811-bc10-423a05842d39")
}

/// `Aboroth` ({4}{G}{G}, 9/9 Creature — Elemental, `Coverage::Partial`) puts an
/// age counter on itself at the beginning of your upkeep and puts a -1/-1
/// counter on itself for each age counter.
#[test]
fn aboroth_upkeep_adds_age_and_minus_one_minus_one_counters() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[aboroth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, aboroth());
    pass_until(&mut engine, stack_is_empty);
    let id = on_battlefield(&engine, p0, aboroth()).unwrap();
    assert_eq!(pt(&engine, id), (9, 9));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert_eq!(pt(&engine, id), (8, 8));
}

/// The cost grows with the age counters (CR 702.24a): upkeep 1 puts one -1/-1
/// counter on (8/8), upkeep 2 two more (6/6), upkeep 3 three more (3/3), and
/// at upkeep 4 four more are too many (toughness -1), so it dies. Nothing
/// happens on the opponent's upkeeps.
#[test]
fn aboroth_shrinks_faster_each_upkeep_and_dies_at_the_fourth() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[aboroth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, aboroth());
    pass_until(&mut engine, stack_is_empty);
    let id = on_battlefield(&engine, p0, aboroth()).unwrap();

    for (before, after) in [((9, 9), (8, 8)), ((8, 8), (6, 6)), ((6, 6), (3, 3))] {
        reach_their_main_phase(&mut engine, p1);
        assert_eq!(
            pt(&engine, id),
            before,
            "the opponent's upkeep changed nothing"
        );
        reach_their_main_phase(&mut engine, p0);
        assert_eq!(pt(&engine, id), after);
    }

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, id), (3, 3));
    reach_their_main_phase(&mut engine, p0);
    assert!(
        in_graveyard(&engine, p0, aboroth()).is_some(),
        "four more -1/-1 counters on a 3/3 is the end of it"
    );
}
