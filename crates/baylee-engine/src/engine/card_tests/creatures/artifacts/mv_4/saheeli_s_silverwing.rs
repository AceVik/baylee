//! `cards/creatures/artifacts/mv_4/saheeli_s_silverwing.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn saheeli_s_silverwing() -> CardIndex {
    card_index("ab5f6e83-a151-48b6-8cc5-3b2dae94079c")
}

/// Saheeli's Silverwing — {4} 2/3 with flying: "When this creature enters,
/// look at the top card of target opponent's library." The controller is
/// shown the opponent's top card, not their own, and the library is left as
/// it was.
#[test]
fn saheelis_silverwing_looks_at_the_opponents_top_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 4])
        .hand(0, &[saheeli_s_silverwing()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let theirs = library_top(&engine, p1, 3);
    let ours = library_top(&engine, p0, 3);

    cast_from_hand(&mut engine, p0, saheeli_s_silverwing());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) || matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    if let Pending::ChooseTargets { .. } = engine.pending() {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p1],
                },
            )
            .expect("the opponent is a legal target");
    }
    pass_until(&mut engine, |e| {
        stack_is_empty(e) || matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange { player, cards, .. } = engine.pending().clone() else {
        panic!("the look is never shown to the controller");
    };
    assert_eq!(player, p0);
    assert_eq!(cards, vec![theirs[0]], "the opponent's top card, alone");
    engine
        .apply(p0, PlayerAction::Arrange { piles: vec![cards] })
        .expect("putting it back where it was");
    pass_until(&mut engine, stack_is_empty);

    let silverwing = on_battlefield(&engine, p0, saheeli_s_silverwing()).expect("it resolved");
    assert_eq!(pt(&engine, silverwing), (2, 3));
    assert!(keywords(&engine, silverwing).contains(KeywordSet::FLYING));
    assert_eq!(library_top(&engine, p1, 3), theirs, "left as it was");
    assert_eq!(library_top(&engine, p0, 3), ours);
}
