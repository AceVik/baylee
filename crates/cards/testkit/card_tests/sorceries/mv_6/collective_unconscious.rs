//! `cards/sorceries/mv_6/collective_unconscious.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn subject_card() -> CardIndex {
    card_index("38abf769-e4ef-4b47-9c79-c8cda0b90f6c")
}

/// Collective Unconscious — {4}{G}{G} Sorcery:
/// "Draw a card for each creature you control."
#[test]
fn collective_unconscious_draws_a_card_for_each_creature_you_control() {
    let mut engine = table(
        SEED,
        &[subject_card()],
        &[forest(), forest(), forest(), forest(), forest(), forest()],
        &[index::GRIZZLY_BEARS, index::LLANOWAR_ELVES],
        &[index::HILL_GIANT],
    );
    let lib_before = library_len(&engine, me());
    cast_plain(&mut engine, subject_card());
    assert_eq!(hand_len(&engine, me()), 2);
    assert_eq!(library_len(&engine, me()), lib_before - 2);
}
