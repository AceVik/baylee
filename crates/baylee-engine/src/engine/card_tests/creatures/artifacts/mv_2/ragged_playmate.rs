//! `cards/creatures/artifacts/mv_2/ragged_playmate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn ragged_playmate() -> CardIndex {
    card_index("d0732ebf-91ad-4e3c-a09a-f410d20587fc")
}

/// Piggy Bank, a 3/2 on the other side of the table: power too great.
fn piggy_bank() -> CardIndex {
    card_index("a13f4095-300b-434f-aa9a-3608aaae5163")
}

/// A Bears attacks into a Wall of Stone; `unblockable` first pays the
/// Playmate's `{1}` and `{T}` to aim "can't be blocked" at the Bears.
fn bears_attack(unblockable: bool) -> Option<Vec<crate::choice::BlockOption>> {
    let p0 = PlayerId::new(0);
    let mut engine = table(
        &[],
        &[ragged_playmate(), grizzly_bears(), forest()],
        &[wall_of_stone()],
    );
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    if unblockable {
        tap_all_mana(&mut engine, p0);
        activate(&mut engine, p0, ragged_playmate(), 0);
        aim_at(&mut engine, p0, bears);
        pass_until(&mut engine, stack_is_empty);
        let playmate = on_battlefield(&engine, p0, ragged_playmate()).expect("seated");
        assert!(is_tapped(&engine, playmate), "{{T}} was part of the price");
    }
    attack_block_options(&mut engine, bears)
}

/// Ragged Playmate: a 2/2 with "{1}, {T}: Target creature with power 2 or less
/// can't be blocked this turn". Aimed at the Bears, the Wall of Stone cannot
/// block them; without it, it can.
#[test]
fn ragged_playmate_makes_a_small_creature_unblockable() {
    let options = bears_attack(true);
    assert!(
        options.as_ref().is_none_or(Vec::is_empty),
        "nobody may block the Bears now: {options:?}"
    );
}

/// The contrast: with no activation the Wall blocks.
#[test]
fn ragged_playmate_without_the_ability_the_wall_may_block() {
    let options = bears_attack(false).expect("a Wall can block, so the question is asked");
    assert!(!options.is_empty(), "the Wall may block the Bears");
}

/// The target filter is "power 2 or less": the Bears and the Playmate are
/// on the menu, a 3-power creature is not.
#[test]
fn ragged_playmate_cannot_target_a_creature_with_power_three() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(
        &[],
        &[ragged_playmate(), grizzly_bears(), forest()],
        &[piggy_bank()],
    );
    let playmate = on_battlefield(&engine, p0, ragged_playmate()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    let piggy = on_battlefield(&engine, p1, piggy_bank()).expect("seated");
    assert_eq!(pt(&engine, piggy), (3, 2));
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, ragged_playmate(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target question")
    };
    assert!(options.contains(&bears) && options.contains(&playmate));
    assert!(
        !options.contains(&piggy),
        "power 3 is too great: {options:?}"
    );
}
