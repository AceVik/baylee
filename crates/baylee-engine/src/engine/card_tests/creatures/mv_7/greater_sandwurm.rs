//! `cards/creatures/mv_7/greater_sandwurm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn greater_sandwurm() -> CardIndex {
    card_index("d1dfed64-6e88-44c2-b479-9a2d072e4be7")
}

/// Greater Sandwurm — a 7/7 for {5}{G}{G}.
#[test]
fn greater_sandwurm_costs_seven_and_is_a_seven_seven() {
    let mut engine = table7(&lots(forest(), 7), &[greater_sandwurm()], &[]);
    let wurm = cast_and_resolve(&mut engine, greater_sandwurm());
    assert_body(&engine, wurm, (7, 7), KeywordSet::default());
}

/// "Can't be blocked by creatures with power 2 or less": a Gray Ogre (power 2)
/// is not offered the block, though it is offered one on an Ogre.
#[test]
fn greater_sandwurm_cannot_be_blocked_by_a_two_power_creature() {
    let (may_block_wurm, may_block_ogre) = ogre_may_block(greater_sandwurm());
    assert!(may_block_ogre, "the control: the Ogre blocks an Ogre");
    assert!(
        !may_block_wurm,
        "power 2 or less may not block the Sandwurm"
    );
}

/// Cycling {2}: two mana discard it from the hand and draw a card.
#[test]
fn greater_sandwurm_cycles_for_two() {
    cycle_it(greater_sandwurm(), &lots(forest(), 2), 0);
}
