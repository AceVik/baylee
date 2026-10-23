//! `cards/artifacts/mv_5/coat_of_arms.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn coat_of_arms() -> CardIndex {
    card_index("5f7f133e-58ea-41ab-b1be-be4b400fac4c")
}

/// Coat of Arms (`5f7f133e-58ea-41ab-b1be-be4b400fac4c`) — {5} Artifact (Partial):
/// "Each creature gets +1/+1 for each other creature on the battlefield that shares at least one creature type with it."
#[test]
fn coat_of_arms_casts_and_enters_battlefield() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[coat_of_arms()])
        .start();
    assert!(walk_to_own_main(&mut engine, P0));

    tap_all_mana(&mut engine, P0);
    cast_from_hand(&mut engine, P0, coat_of_arms());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, coat_of_arms()).is_some());
}
