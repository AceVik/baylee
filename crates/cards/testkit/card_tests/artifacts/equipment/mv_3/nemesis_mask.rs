//! `cards/artifacts/equipment/mv_3/nemesis_mask.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn nemesis_mask() -> CardIndex {
    card_index("34062d79-457f-4af3-b9c6-cab2e4d17581")
}

/// Nemesis Mask: "All creatures able to block equipped creature do so.
/// Equip {3}". Both of their creatures are drafted into blocking the host.
#[test]
fn nemesis_mask_forces_every_able_creature_to_block() {
    let mut r = rig_vs(
        nemesis_mask(),
        jasmine_boreal(),
        3,
        &[],
        &[goblin_piker(), llanowar_elves()],
        &[],
    );
    r.equip(3);
    let Pending::ChooseBlockers {
        blockers, obeying, ..
    } = r.attack_to_the_blocks()
    else {
        unreachable!("to_the_blocks returns the blockers question")
    };
    assert_eq!(blockers.len(), 2, "both of their creatures can block");
    assert_eq!(obeying.len(), 2, "and both are required to: {obeying:?}");
    assert!(
        obeying.iter().all(|(_, attacker)| *attacker == r.host),
        "all against the equipped creature: {obeying:?}"
    );
}
