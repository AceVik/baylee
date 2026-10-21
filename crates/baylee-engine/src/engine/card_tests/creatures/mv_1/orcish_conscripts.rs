//! `cards/creatures/mv_1/orcish_conscripts.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn orcish_conscripts() -> CardIndex {
    card_index("d99e1d08-32ca-49c5-97cc-e064d661202f")
}

/// Orcish Conscripts (d99e1d08-32ca-49c5-97cc-e064d661202f) — {R} 2/2 Creature — Orc:
/// Coverage::Partial — casts for {R} as a 2/2 Orc.
#[test]
fn orcish_conscripts_casts_for_red_as_a_two_two_orc() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain()])
        .hand(0, &[orcish_conscripts()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, orcish_conscripts());
    pass_until(&mut engine, stack_is_empty);

    let orc = on_battlefield(&engine, P0, orcish_conscripts()).unwrap();
    assert_eq!(pt(&engine, orc), (2, 2));
}
