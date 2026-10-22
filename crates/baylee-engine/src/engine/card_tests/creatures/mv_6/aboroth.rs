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
