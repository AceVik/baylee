//! `cards/creatures/artifacts/mv_2/doom_s_servo_guards.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn doom_s_servo_guards() -> CardIndex {
    card_index("296e79f9-57cb-4af7-9e18-48d2d655b537")
}

/// Doom's Servo-Guards: a 2/2 with "When this creature enters, you gain 2
/// life and mill two cards." Cast for `{1}{B}`, its controller ends on 22
/// life with two cards moved from the library to the graveyard.
#[test]
fn doom_s_servo_guards_gains_two_life_and_mills_two_when_it_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[doom_s_servo_guards()], &[swamp(), forest()], &[]);
    let guards = cast_and_land(&mut engine, doom_s_servo_guards());
    assert_eq!(pt(&engine, guards), (2, 2));
    let library = library_size(&engine, p0);
    let graveyard = graveyard_count(&engine, 0);
    let theirs = library_size(&engine, PlayerId::new(1));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, 0), 22);
    assert_eq!(library_size(&engine, p0), library - 2, "two milled");
    assert_eq!(graveyard_count(&engine, 0), graveyard + 2);
    assert_eq!(
        library_size(&engine, PlayerId::new(1)),
        theirs,
        "the opponent's library is untouched"
    );
    assert_eq!(life_of(&engine, 1), 20);
}
