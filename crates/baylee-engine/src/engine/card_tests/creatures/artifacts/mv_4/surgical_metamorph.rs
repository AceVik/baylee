//! `cards/creatures/artifacts/mv_4/surgical_metamorph.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Surgical Metamorph: "You may have Surgical Metamorph enter as a copy of
/// any permanent on the battlefield, except it's an artifact in addition to
/// its other types."
#[test]
fn surgical_metamorph_enters_as_a_copy_that_is_also_an_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(4609, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[surgical_metamorph()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("their Bears");
    cast_from_hand(&mut engine, p0, surgical_metamorph());
    for _ in 0..6 {
        match engine.pending().clone() {
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(true)).unwrap();
            }
            Pending::ChooseTargets { player, .. } | Pending::ChooseCards { player, .. } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![bears],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { player, .. } => {
                if stack_is_empty(&engine) {
                    break;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
    }
    let copy = on_battlefield(&engine, p0, surgical_metamorph()).expect("my Metamorph entered");
    assert_eq!(pt(&engine, copy), (2, 2), "a copy of the Bears");
    let kinds = types(&engine, copy);
    assert!(kinds.contains(TypeSet::CREATURE) && kinds.contains(TypeSet::ARTIFACT));
}
