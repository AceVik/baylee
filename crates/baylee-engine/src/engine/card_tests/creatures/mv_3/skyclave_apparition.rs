//! `cards/creatures/mv_3/skyclave_apparition.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "Exile **up to one** target ... you don't control" against a board with
/// nothing on it to exile.
///
/// The trigger still goes on the stack. CR 603.3d removes a triggered
/// ability that cannot be given a legal target, and one that requires no
/// target always can be — so what it must not do is *ask*: with an empty
/// option list and a minimum of zero, the only answer is the empty list,
/// and a stop the player cannot influence is not a choice. The engine used
/// to publish `ChooseTargets { options: [], min: 0, max: 0 }` and wait
/// there.
///
/// The second board is what keeps the first honest. A filter that matched
/// nothing at all would pass the first half and read exactly the same, so
/// the same Apparition is put down against a creature it *can* exile and
/// the question has to appear.
#[test]
fn up_to_one_target_with_nothing_to_point_at_is_not_a_question() {
    let p0 = PlayerId::new(0);
    let mut engine = a_skyclave_over(&[forest(), forest()]);
    let mut trigger_stacked = false;
    for _ in 0..20 {
        if stack_is_empty(&engine) {
            break;
        }
        // Something on the stack while the Apparition is already standing is
        // its own enters-trigger: the spell has left, and nothing else at
        // this table triggers at all. Without this the test would read the
        // same on a trigger CR 603.3d had *removed* — an exile that finds
        // nothing to exile does nothing either way, so "nobody was asked" is
        // only half of what is being claimed.
        if on_battlefield(&engine, p0, skyclave_apparition()).is_some() {
            trigger_stacked = true;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("nothing was legal to exile, so nothing was asked: {other:?}"),
        }
    }
    assert!(
        trigger_stacked,
        "the trigger went on the stack, with no targets and no question"
    );
    assert!(stack_is_empty(&engine), "and then resolved");
    assert!(
        on_battlefield(&engine, p0, skyclave_apparition()).is_some(),
        "and the Apparition itself is standing there, trigger and all"
    );

    let mut engine = a_skyclave_over(&[forest(), ondu_cleric()]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, min, .. } = engine.pending().clone() else {
        unreachable!("the loop above stopped on one")
    };
    assert_eq!(min, 0, "\"up to one\" may still decline");
    assert_eq!(
        options,
        vec![on_battlefield(&engine, PlayerId::new(1), ondu_cleric()).expect("their Cleric")],
        "their Cleric is the one thing it may point at"
    );
}
