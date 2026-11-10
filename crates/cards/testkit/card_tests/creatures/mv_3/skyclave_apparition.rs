//! `cards/creatures/mv_3/skyclave_apparition.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "Exile **up to one** target ... you don't control" against a board with
/// nothing on it to exile.
///
/// CR 603.3d removes a triggered ability that cannot be given a legal target.
/// The table's empty-optional policy also covers this minimum-zero case: the
/// empty answer is proved to have no effect, so the trigger leaves without an
/// empty resolution and without a meaningless question. The journal separates
/// that removal from a trigger that never fired or resolved normally.
///
/// The second board is what keeps the first honest. A filter that matched
/// nothing at all would pass the first half and read exactly the same, so
/// the same Apparition is put down against a creature it *can* exile and
/// the question has to appear.
#[test]
fn up_to_one_target_with_nothing_to_point_at_is_removed_without_a_question() {
    let p0 = PlayerId::new(0);
    let mut engine = a_skyclave_over(&[forest(), forest()]);
    pass_until(&mut engine, stack_is_empty);
    let source =
        on_battlefield(&engine, p0, skyclave_apparition()).expect("the Apparition entered");
    let trigger = engine
        .journal()
        .entries()
        .iter()
        .find_map(|entry| match entry.event {
            GameEvent::AbilityTriggered {
                object,
                source: from,
                ..
            } if from == source => Some(object),
            _ => None,
        })
        .expect("the Apparition's enters trigger");
    assert!(
        engine.journal().entries().iter().any(|entry| matches!(
            entry.event,
            GameEvent::StackObjectDidNotResolve { object } if object == trigger
        )),
        "the empty optional trigger was removed without resolving"
    );
    assert!(
        !engine.journal().entries().iter().any(|entry| matches!(
            entry.event,
            GameEvent::StackObjectResolved { object } if object == trigger
        )),
        "and it did not resolve as an empty ability"
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
