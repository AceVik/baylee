//! `cards/creatures/mv_2/skyshroud_condor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn skyshroud_condor() -> CardIndex {
    card_index("c7c4c67e-c4db-416b-b2e8-f0e59aa80bba")
}

/// `Skyshroud Condor` ({1}{U}, 2/2 Creature — Bird, `Coverage::Partial`):
/// casts and enters as a 2/2 with flying.
#[test]
fn skyshroud_condor_casts_and_enters_with_flying() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[skyshroud_condor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, skyshroud_condor());
    pass_until(&mut engine, stack_is_empty);

    let condor = on_battlefield(&engine, p0, skyshroud_condor()).unwrap();
    let ch = engine.state().object(condor).unwrap().characteristics();
    assert!(ch.keywords.contains(KeywordSet::FLYING));
    assert_eq!((ch.power, ch.toughness), (Some(2), Some(2)));
}
