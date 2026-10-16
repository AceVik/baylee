//! `cards/instants/mv_5/unfathomable_truths.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "63765157-e2ba-4cec-9501-8a405259de5b"
fn unfathomable_truths() -> CardIndex {
    card_index("63765157-e2ba-4cec-9501-8a405259de5b")
}

/// Unfathomable Truths: "Devoid. Draw three cards and create a 0/1 colorless
/// Eldrazi Spawn creature token with 'Sacrifice this token: Add {C}.'"
#[test]
fn unfathomable_truths_draws_three_and_makes_a_spawn() {
    let mut engine = table(936, &[unfathomable_truths()], &[island(); 5], &[], &[]);
    let (hand_before, library_before) = (hand_len(&engine, me()), library_len(&engine, me()));
    cast_plain(&mut engine, unfathomable_truths());
    assert_eq!(hand_len(&engine, me()), hand_before - 1 + 3);
    assert_eq!(library_len(&engine, me()), library_before - 3);
    let tokens = my_tokens(&engine);
    assert_eq!(tokens.len(), 1, "one Spawn");
    assert_eq!(pt(&engine, tokens[0]), (0, 1));
    assert!(types(&engine, tokens[0]).contains(TypeSet::CREATURE));
}
