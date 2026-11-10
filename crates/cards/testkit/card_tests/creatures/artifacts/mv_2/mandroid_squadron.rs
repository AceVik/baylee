//! `cards/creatures/artifacts/mv_2/mandroid_squadron.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn mandroid_squadron() -> CardIndex {
    card_index("95ab1529-0de0-416a-82b4-c5ed9276a9b4")
}

/// Mandroid Squadron: a 0/4 that, "when this creature enters, you gain 2
/// life". Cast for `{1}{W}`: 22 life once the trigger resolves, not before.
#[test]
fn mandroid_squadron_gains_two_life_when_it_enters() {
    let mut engine = table(&[mandroid_squadron()], &[plains(), forest()], &[]);
    let squad = cast_and_land(&mut engine, mandroid_squadron());
    assert_eq!(pt(&engine, squad), (0, 4));
    assert_eq!(life_of(&engine, 0), 20, "the trigger has not resolved yet");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, 0), 22);
    assert_eq!(life_of(&engine, 1), 20, "only its controller gains");
}

/// Only entering counts: a Squadron seated at the start has gained nothing.
#[test]
fn mandroid_squadron_seated_from_the_start_gains_nothing() {
    let engine = table(&[], &[mandroid_squadron()], &[]);
    assert_eq!(life_of(&engine, 0), 20);
}
