//! `cards/artifacts/mv_2/arcum_s_weathervane.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn arcum_s_weathervane() -> CardIndex {
    card_index("b4e57fba-5fa3-4fd5-8f0a-7b61593ade72")
}

/// Arcum's Weathervane (b4e57fba-5fa3-4fd5-8f0a-7b61593ade72) — {2} Artifact
/// (`Coverage::Partial`: adding or removing the Snow supertype is not supported).
#[test]
fn arcum_s_weathervane_casts_for_two_and_enters_the_battlefield() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[arcum_s_weathervane()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, arcum_s_weathervane());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, arcum_s_weathervane()).is_some());
}
