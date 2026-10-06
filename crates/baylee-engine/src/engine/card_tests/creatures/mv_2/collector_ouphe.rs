//! `cards/creatures/mv_2/collector_ouphe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Collector Ouphe prints one sentence — "Activated abilities of artifacts
/// can't be activated" — and the card carries **no ability at all**:
/// `Modifier::CantActivateArtifacts` reaches the artifacts an effect's
/// opponents control (it was written for Karn, the Great Creator), and the
/// DSL has no modifier whose reach is every artifact, its controller's
/// included. So the file is `Coverage::Partial` with the whole of its text
/// in the reason.
///
/// This pins that. An artifact's activated ability is activated with the
/// Ouphe on the battlefield and it works, which is the defect written down
/// rather than left to be discovered in a game — and the assertion is
/// **meant to fail** the day the modifier exists. The printed body is
/// checked beside it, because a card that is only a 2/2 should at least be
/// the right 2/2.
#[test]
fn collector_ouphe_is_a_body_and_its_only_sentence_is_not_built_yet() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[collector_ouphe(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let body = on_battlefield(&engine, p0, collector_ouphe()).expect("the Ouphe is in play");
    assert_eq!(pt(&engine, body), (2, 2), "a 2/2 Ouphe is the printed body");

    let before = engine.state().players[0].mana_pool.total();
    activate(&mut engine, p0, quiet_artifact(), 0);
    assert!(
        engine.state().players[0].mana_pool.total() > before,
        "an artifact's activated ability still works with Collector Ouphe on \
         the battlefield: the card's only sentence is not built, and this is \
         the assertion that has to be deleted when it is"
    );
}
