//! `cards/enchantments/mv_1/high_ground.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn high_ground() -> CardIndex {
    card_index("b59053d5-b5f2-44aa-a29c-7f3908d86577")
}

/// "Each creature you control can block an additional creature each combat."
#[test]
fn high_ground_lets_a_blocker_take_two_attackers() {
    for (seed, with) in [(7171, true), (7172, false)] {
        let mine: &[CardIndex] = &[index::GRIZZLY_BEARS, index::GRIZZLY_BEARS];
        let theirs: Vec<CardIndex> = if with {
            vec![high_ground(), index::HILL_GIANT]
        } else {
            vec![index::HILL_GIANT]
        };
        let mut e = table(seed, &[], mine, &theirs);
        let attackers = loop_to_attack(&mut e);
        let Pending::ChooseBlockers { capacity, .. } = e.pending().clone() else {
            panic!("blockers expected, got {:?}", e.pending())
        };
        let giant = on_battlefield(&e, P1, index::HILL_GIANT).unwrap();
        assert_eq!(attackers.len(), 2);
        assert_eq!(
            capacity
                .iter()
                .any(|c| format!("{c:?}").contains(&format!("{giant:?}"))),
            with,
            "{capacity:?}"
        );
    }
}

/// Declares both of seat 0's creatures as attackers on seat 1 and passes
/// priority until seat 1 is asked for blockers.
fn loop_to_attack(e: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    pass_until(e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = e.pending().clone() else {
        unreachable!()
    };
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: attackers
                .iter()
                .map(|a| (*a, baylee_core::ids::Defender::Player(P1)))
                .collect(),
        },
    )
    .unwrap();
    pass_until(e, |e| matches!(e.pending(), Pending::ChooseBlockers { .. }));
    attackers
}
