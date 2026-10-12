//! Additional acceptance edges for this card.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn valakut_trigger_counts_five_remaining_mountains_after_entering_one_leaves() {
    let mut e = prepared(
        &[index::MOUNTAIN, index::BOOMERANG],
        &[
            index::VALAKUT_THE_MOLTEN_PINNACLE,
            index::MOUNTAIN,
            index::MOUNTAIN,
            index::MOUNTAIN,
            index::MOUNTAIN,
            index::MOUNTAIN,
        ],
        &[],
    );
    let entering = in_hand(&e, P0, index::MOUNTAIN).unwrap();
    let before = e.state.players[1].life;
    play_land(&mut e, P0, index::MOUNTAIN);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    targets(&mut e, vec![], vec![P1]);
    cast(&mut e, index::BOOMERANG);
    targets(&mut e, vec![entering], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(entering).unwrap().zone, Zone::Hand);
    assert_eq!(e.state.players[1].life, before - 3);
}

#[test]
fn valakut_old_trigger_does_not_exclude_a_reentered_incarnation() {
    let mut e = prepared(
        &[index::MOUNTAIN],
        &[
            index::VALAKUT_THE_MOLTEN_PINNACLE,
            index::MOUNTAIN,
            index::MOUNTAIN,
            index::MOUNTAIN,
            index::MOUNTAIN,
            index::MOUNTAIN,
        ],
        &[],
    );
    let entering = in_hand(&e, P0, index::MOUNTAIN).unwrap();
    let old_other = source(&e, P0, index::MOUNTAIN);
    let before = e.state.players[1].life;
    play_land(&mut e, P0, index::MOUNTAIN);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    targets(&mut e, vec![], vec![P1]);
    let version = e.state.object(entering).unwrap().version;
    // Explicit incarnation fixture: the old trigger was created and targeted
    // through a real land play. Keep it pending while replacing its event
    // incarnation. At the next collection the reentry itself has only four
    // other Mountains, so it cannot create a second damage trigger.
    e.state
        .move_object(
            entering,
            ZoneLocation::Exile(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state
        .move_object(
            entering,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state
        .move_object(
            old_other,
            ZoneLocation::Hand(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    assert_ne!(e.state.object(entering).unwrap().version, version);
    // Four original Mountains plus the new incarnation satisfy five OTHER
    // Mountains relative to the old event, despite retaining the same ObjectId.
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[1].life, before - 3);
}

#[test]
fn valakut_requires_a_target_then_allows_declining_damage_on_resolution() {
    let mut e = prepared(
        &[index::MOUNTAIN],
        &[
            index::VALAKUT_THE_MOLTEN_PINNACLE,
            index::MOUNTAIN,
            index::MOUNTAIN,
            index::MOUNTAIN,
            index::MOUNTAIN,
            index::MOUNTAIN,
        ],
        &[],
    );
    let before = e.state.players[1].life;
    play_land(&mut e, P0, index::MOUNTAIN);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let before_rejection = e.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![]
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), before_rejection);
    targets(&mut e, vec![], vec![P1]);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    yes(&mut e, false);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[1].life, before);
}
