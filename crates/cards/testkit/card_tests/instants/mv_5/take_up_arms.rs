//! `cards/instants/mv_5/take_up_arms.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "61e2d557-f08d-489b-94c8-aff87e5189f9"
fn take_up_arms() -> CardIndex {
    card_index("61e2d557-f08d-489b-94c8-aff87e5189f9")
}

/// Take Up Arms: "Create three 1/1 white Warrior creature tokens."
#[test]
fn take_up_arms_creates_three_warriors() {
    let mut engine = table(931, &[take_up_arms()], &[plains(); 5], &[], &[]);
    assert!(my_tokens(&engine).is_empty());
    cast_plain(&mut engine, take_up_arms());
    let tokens = my_tokens(&engine);
    assert_eq!(tokens.len(), 3, "three tokens");
    for token in tokens {
        assert_eq!(pt(&engine, token), (1, 1));
        assert!(types(&engine, token).contains(TypeSet::CREATURE));
    }
    assert!(
        tokens_of(&engine, them()).is_empty(),
        "the opponent gets none"
    );
}
