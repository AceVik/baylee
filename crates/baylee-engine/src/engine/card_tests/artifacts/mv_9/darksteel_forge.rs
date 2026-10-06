//! `cards/artifacts/mv_9/darksteel_forge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// A permanent that enters under a static grant is projected against it.
///
/// The projection cache is keyed on the *effect table's* generation, and a
/// permanent arriving changes no effect: so the refresh pass at the top of
/// the machine took its early exit, and the newcomer kept the cleared cache
/// `move_object` left it — which reads as the printed card. Darksteel Forge
/// says artifacts you control have indestructible, and a Sol Ring cast into
/// that board had none of it.
///
/// The board is the smaller half of the claim: the Forge is on the
/// battlefield before the game starts, so its static is registered and the
/// generation has been still ever since. Nothing but the arrival is left to
/// account for the difference.
#[test]
fn a_permanent_that_enters_under_a_static_grant_is_projected_against_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(37, forest())
        .battlefield(0, &[darksteel_forge(), forest()])
        .hand(0, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, quiet_artifact()).is_some()
    });
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("the Ring resolved");
    assert!(
        engine
            .state()
            .object(ring)
            .expect("the Ring is an object")
            .characteristics()
            .keywords
            .contains(KeywordSet::INDESTRUCTIBLE),
        "the Forge grants indestructible to artifacts that arrive after it too",
    );
}
