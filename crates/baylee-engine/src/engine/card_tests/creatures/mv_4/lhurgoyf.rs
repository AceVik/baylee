//! `cards/creatures/mv_4/lhurgoyf.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn lhurgoyf() -> CardIndex {
    card_index("0fe76004-b2f4-4b11-abce-75fd76f68b4d")
}

/// Lhurgoyf (0fe76004-b2f4-4b11-abce-75fd76f68b4d) — {2}{G}{G} Creature — Lhurgoyf:
/// Coverage::Partial — plays as a 0/1 creature.
#[test]
fn lhurgoyf_casts_for_two_and_two_green_and_enters_as_a_zero_one_lhurgoyf() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[lhurgoyf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, lhurgoyf());
    pass_until(&mut engine, stack_is_empty);

    let goyf = on_battlefield(&engine, P0, lhurgoyf()).expect("Lhurgoyf resolved onto battlefield");
    assert_eq!(pt(&engine, goyf), (0, 1));
}
