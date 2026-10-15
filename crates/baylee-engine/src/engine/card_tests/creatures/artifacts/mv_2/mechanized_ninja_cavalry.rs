//! `cards/creatures/artifacts/mv_2/mechanized_ninja_cavalry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn mechanized_ninja_cavalry() -> CardIndex {
    card_index("2db8dda3-8136-4cf2-ba2d-ecc344a36725")
}

/// Mechanized Ninja Cavalry: a 1/1 `{1}{R/W}` that, "when this creature
/// enters, create a 1/1 colorless Robot artifact creature token". Cast off a
/// Plains and a Forest, it brings exactly one token.
#[test]
fn mechanized_ninja_cavalry_creates_a_one_one_robot_token_when_it_enters() {
    use baylee_core::generated::subtypes::creature;
    let p0 = PlayerId::new(0);
    let mut engine = table(&[mechanized_ninja_cavalry()], &[plains(), forest()], &[]);
    let cavalry = cast_and_land(&mut engine, mechanized_ninja_cavalry());
    assert_eq!(pt(&engine, cavalry), (1, 1));
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "the trigger has not resolved"
    );
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1);
    let token = tokens[0];
    assert_eq!(pt(&engine, token), (1, 1));
    let chars = engine.state().object(token).unwrap().characteristics();
    assert!(chars.types.contains(TypeSet::ARTIFACT));
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(chars.colors.is_empty(), "colorless");
    assert!(chars.subtypes.contains(creature::ROBOT));
}

/// Hybrid `{R/W}` is paid with red as well: a Mountain and a Forest cast it.
#[test]
fn mechanized_ninja_cavalry_may_be_cast_with_red() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[mechanized_ninja_cavalry()], &[mountain(), forest()], &[]);
    cast_and_land(&mut engine, mechanized_ninja_cavalry());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, mechanized_ninja_cavalry()).is_some());
    assert_eq!(tokens_of(&engine, p0).len(), 1);
}

/// Without red or white the hybrid symbol cannot be paid.
#[test]
fn mechanized_ninja_cavalry_cannot_be_cast_with_only_green() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[mechanized_ninja_cavalry()], &[forest(), forest()], &[]);
    tap_all_mana(&mut engine, p0);
    let card = in_hand(&engine, p0, mechanized_ninja_cavalry()).expect("in hand");
    assert!(
        engine.apply(p0, PlayerAction::CastSpell { card }).is_err(),
        "green does not pay {{R/W}}"
    );
}
