//! `cards/enchantments/mv_1/elven_fortress.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn elven_fortress() -> CardIndex {
    card_index("29f34e18-2102-4ded-912f-4ac3e9e6744c")
}

/// "{1}{G}: Target blocking creature gets +0/+1 until end of turn."
#[test]
fn elven_fortress_pumps_a_blocking_creature_only() {
    let mut e = table(
        7281,
        &[],
        &[index::HILL_GIANT],
        &[elven_fortress(), index::GRIZZLY_BEARS],
    );
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = e.pending().clone() else {
        unreachable!()
    };
    let giant = attackers[0];
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(giant, baylee_core::ids::Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let bears = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(bears, giant)],
        },
    )
    .unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    fill_pool(&mut e);
    assert_eq!(pt(&e, bears), (2, 2));
    activate(&mut e, P1, elven_fortress(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("target question expected, got {:?}", e.pending())
    };
    assert!(options.contains(&bears), "the blocker is a target");
    assert!(!options.contains(&giant), "the attacker is not blocking");
    drive(&mut e, &[bears], &[], &[], true);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, bears), (2, 3));
}

/// With nothing blocking there is no target for it.
#[test]
fn elven_fortress_has_no_target_outside_a_block() {
    let mut e = table(7282, &[], &[], &[elven_fortress(), index::GRIZZLY_BEARS]);
    // Seat 1's ability is only offered to seat 1; reach its priority.
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    fill_pool(&mut e);
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority expected")
    };
    assert!(
        legal.abilities.is_empty(),
        "no blocking creature, no activation: {:?}",
        legal.abilities
    );
}
