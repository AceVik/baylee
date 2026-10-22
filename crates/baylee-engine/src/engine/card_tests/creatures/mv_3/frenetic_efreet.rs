//! `cards/creatures/mv_3/frenetic_efreet.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::FRENETIC_EFREET;

const P0: PlayerId = PlayerId::new(0);

/// Frenetic Efreet (cc7993c6-4137-49f1-983b-b88f27a1dbce) — {1}{U}{R} 2/1:
/// "Flying. {0}: Flip a coin. If you win the flip, this creature phases out.
/// If you lose the flip, sacrifice this creature."
#[test]
fn frenetic_efreet_flips_coin_to_phase_out_or_be_sacrificed() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[FRENETIC_EFREET])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let efreet = on_battlefield(&engine, P0, FRENETIC_EFREET).unwrap();
    assert!(
        engine
            .state()
            .object(efreet)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING)
    );

    activate(&mut engine, P0, FRENETIC_EFREET, 0);
    pass_until(&mut engine, stack_is_empty);

    let phased_out = engine
        .state()
        .object(efreet)
        .is_some_and(|o| o.status.contains(Status::PHASED_OUT));
    let sacrificed = on_battlefield(&engine, P0, FRENETIC_EFREET).is_none();
    assert!(
        phased_out || sacrificed,
        "Frenetic Efreet must either phase out or be sacrificed"
    );
}
