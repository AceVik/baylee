//! Oracle scenarios for Agony Warp.
#[allow(clippy::wildcard_imports)]
use super::*;

fn agony_warp() -> CardIndex {
    card_index("377c217b-ddad-4e1a-9122-f347113d3dfa")
}

// Checklist: each clause asks for a target; the same or different creatures
// are legal; each shrink is exactly -3 in one stat; partial illegality leaves
// the other clause working; effects expire at end of turn.
#[test]
fn agony_warp_can_target_the_same_creature_for_both_shrinks_and_expires() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[island(), swamp()])
        .hand(0, &[agony_warp()])
        .battlefield(1, &[fangren_hunter()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let target = on_battlefield(&e, p1, fangren_hunter()).unwrap();
    cast_from_hand(&mut e, p0, agony_warp());
    for clause in 0..2 {
        let Pending::ChooseTargets {
            options, min, max, ..
        } = e.pending().clone()
        else {
            panic!("expected target for clause {clause}")
        };
        assert_eq!((min, max), (1, 1));
        assert!(
            options.contains(&target),
            "the same target is legal for both target instances"
        );
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
    }
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, target), (1, 1), "both -3 clauses apply to the 4/4");
    reach_their_main_phase(&mut e, p1);
    assert_eq!(pt(&e, target), (4, 4), "both effects expire at end of turn");
}

#[test]
fn agony_warp_can_use_different_targets_and_keeps_the_legal_clause() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[island(), swamp()])
        .hand(0, &[agony_warp()])
        .battlefield(1, &[island(), fangren_hunter(), fangren_hunter()])
        .hand(1, &[unsummon()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let mut targets: Vec<_> = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            e.state().object(*id).is_some_and(|o| {
                o.controller == p1 && o.card.is_some_and(|c| c.index == fangren_hunter())
            })
        })
        .collect();
    targets.sort_unstable();
    assert_eq!(targets.len(), 2);
    let first = targets[0];
    let second = targets[1];
    cast_from_hand(&mut e, p0, agony_warp());
    for target in [first, second] {
        let Pending::ChooseTargets {
            options, min, max, ..
        } = e.pending().clone()
        else {
            panic!("one target per clause")
        };
        assert_eq!((min, max), (1, 1));
        assert!(options.contains(&target));
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
    }
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut e, p1, unsummon());
    e.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![first],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_hand(&e, p1, fangren_hunter()).is_some());
    assert_eq!(
        pt(&e, second),
        (4, 1),
        "the still-legal second target gets -0/-3"
    );
}
