//! Child of `transforms_remaining`. Independent trigger payment after source loss.
#[allow(clippy::wildcard_imports)]
use super::*;
#[test]
fn smithy_first_main_trigger_can_pay_other_five_after_source_leaves() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[
                index::THOUSAND_MOONS_SMITHY,
                index::SOL_RING,
                index::SOL_RING,
                index::GRIZZLY_BEARS,
                index::GRIZZLY_BEARS,
                index::ORNITHOPTER,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let smithy = source(&e, P0, index::THOUSAND_MOONS_SMITHY);
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    let other_five: Vec<_> = options.into_iter().filter(|id| *id != smithy).collect();
    assert_eq!(other_five.len(), 5);
    // Transparent boundary fixture: remove source while payment continuation
    // exists; priority is not fabricated inside the resolving trigger.
    e.state
        .move_object(
            smithy,
            ZoneLocation::Hand(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    choose(&mut e, other_five.clone());
    pass_until(&mut e, stack_is_empty);
    assert!(
        other_five.iter().all(|id| is_tapped(&e, *id)),
        "independent trigger still permits valid payment"
    );
    assert_eq!(e.state.object(smithy).unwrap().zone, Zone::Hand);
    assert_eq!(
        face(&e, smithy),
        0,
        "old trigger cannot transform departed/new source incarnation"
    );
}
