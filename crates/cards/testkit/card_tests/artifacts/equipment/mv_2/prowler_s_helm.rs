//! `cards/artifacts/equipment/mv_2/prowler_s_helm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn prowler_s_helm() -> CardIndex {
    card_index("603ef53f-6191-4229-b85f-adad6f4503ce")
}

/// Prowler's Helm: "Equipped creature can't be blocked except by Walls.
/// Equip {2}". Across the table stand a Wall and a plain Elf; only the Wall
/// is offered as a blocker.
#[test]
fn prowler_s_helm_can_only_be_blocked_by_walls() {
    let mut r = rig_vs(
        prowler_s_helm(),
        llanowar_elves(),
        2,
        &[],
        &[llanowar_elves(), steel_wall()],
        &[],
    );
    r.equip(2);
    let wall = on_battlefield(&r.engine, PlayerId::new(1), steel_wall()).expect("their Wall");
    let Pending::ChooseBlockers { blockers, .. } = r.attack_to_the_blocks() else {
        unreachable!("to_the_blocks returns the blockers question")
    };
    let wall_offer = blockers
        .iter()
        .find(|b| b.blocker == wall)
        .expect("a Wall may block it");
    assert!(wall_offer.attackers.contains(&r.host));
    assert!(
        blockers
            .iter()
            .filter(|b| b.blocker == r.theirs)
            .all(|b| !b.attackers.contains(&r.host)),
        "the Elf may not: {blockers:?}"
    );
}
