//! `cards/creatures/artifacts/mv_2/suspicious_bookcase.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn suspicious_bookcase() -> CardIndex {
    card_index("ca611c90-7753-43dd-af91-81694d004eeb")
}

/// A Bears attacks into a Wall of Stone; `unblockable` first pays the
/// Bookcase's `{3}` and `{T}` to aim "can't be blocked" at the Bears.
fn bears_attack(unblockable: bool) -> Option<Vec<crate::choice::BlockOption>> {
    let p0 = PlayerId::new(0);
    let mut engine = table(
        &[],
        &[
            suspicious_bookcase(),
            grizzly_bears(),
            forest(),
            forest(),
            forest(),
        ],
        &[wall_of_stone()],
    );
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    if unblockable {
        tap_all_mana(&mut engine, p0);
        activate(&mut engine, p0, suspicious_bookcase(), 0);
        aim_at(&mut engine, p0, bears);
        pass_until(&mut engine, stack_is_empty);
        let bookcase = on_battlefield(&engine, p0, suspicious_bookcase()).expect("seated");
        assert!(is_tapped(&engine, bookcase), "{{T}} was part of the price");
    }
    attack_block_options(&mut engine, bears)
}

/// Suspicious Bookcase: a 0/4 with "Defender" and "{3}, {T}: Target creature
/// can't be blocked this turn". Aimed at the Bears, the Wall cannot block.
#[test]
fn suspicious_bookcase_makes_a_target_creature_unblockable() {
    let options = bears_attack(true);
    assert!(
        options.as_ref().is_none_or(Vec::is_empty),
        "nobody may block the Bears now: {options:?}"
    );
}

/// The contrast: with no activation the Wall blocks.
#[test]
fn suspicious_bookcase_without_the_ability_the_wall_may_block() {
    let options = bears_attack(false).expect("a Wall can block, so the question is asked");
    assert!(!options.is_empty());
}

/// Defender: it is not offered as an attacker.
#[test]
fn suspicious_bookcase_cannot_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[suspicious_bookcase(), grizzly_bears()], &[]);
    let bookcase = on_battlefield(&engine, p0, suspicious_bookcase()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    assert_eq!(pt(&engine, bookcase), (0, 4));
    assert!(keywords(&engine, bookcase).contains(KeywordSet::DEFENDER));
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(attackers.contains(&bears));
    assert!(!attackers.contains(&bookcase));
}

/// "Target creature": the opponent's Wall is on the menu as well as ours.
#[test]
fn suspicious_bookcase_may_target_any_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(
        &[],
        &[
            suspicious_bookcase(),
            grizzly_bears(),
            forest(),
            forest(),
            forest(),
        ],
        &[wall_of_stone()],
    );
    let wall = on_battlefield(&engine, p1, wall_of_stone()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, suspicious_bookcase(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target question")
    };
    assert!(options.contains(&wall) && options.contains(&bears));
}
