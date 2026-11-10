//! `cards/artifacts/equipment/mv_2/vanguard_s_shield.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn vanguard_s_shield() -> CardIndex {
    card_index("bc628018-f5d0-4bd2-91e7-ee4e4a5af9aa")
}

/// Vanguard's Shield: "Equipped creature gets +0/+3 and can block an
/// additional creature each combat. Equip {3}".
#[test]
fn vanguard_s_shield_adds_toughness_and_a_second_block() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut r = rig_vs(
        vanguard_s_shield(),
        llanowar_elves(),
        3,
        &[],
        &[goblin_piker(), goblin_piker()],
        &[],
    );
    r.equip(3);
    assert_eq!(pt(&r.engine, r.host), (1, 4), "+0/+3");
    let attackers = all_on_battlefield(&r.engine, p1, goblin_piker());
    assert_eq!(attackers.len(), 2);
    let Pending::ChooseBlockers { capacity, .. } = r.defend_against(&attackers) else {
        unreachable!("defend_against returns the blockers question")
    };
    assert!(
        capacity
            .iter()
            .any(|c| c.blocker == r.host && c.most == Some(2)),
        "the equipped creature may block two: {capacity:?}"
    );
    assert!(
        capacity.iter().all(|c| c.blocker != r.bystander),
        "the other one blocks a single attacker: {capacity:?}"
    );
    r.engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(r.host, attackers[0]), (r.host, attackers[1])],
            },
        )
        .expect("blocking both is legal");
}
