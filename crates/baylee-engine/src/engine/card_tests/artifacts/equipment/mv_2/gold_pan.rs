//! `cards/artifacts/equipment/mv_2/gold_pan.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn gold_pan() -> CardIndex {
    card_index("91415d12-b13d-4b5c-bb60-6ec6f4fdde6e")
}

/// Gold Pan: "Equipped creature gets +1/+1. Equip {1}".
#[test]
fn gold_pan_gives_plus_one_plus_one() {
    equipped_gets(gold_pan(), 1, llanowar_elves(), (1, 1), KeywordSet::EMPTY);
}

/// "When this Equipment enters, create a Treasure token."
#[test]
fn gold_pan_entering_creates_a_treasure() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[gold_pan()])
        .battlefield(0, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "no token before the cast"
    );
    cast_from_hand(&mut engine, p0, gold_pan());
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "exactly one token");
    assert!(types(&engine, tokens[0]).contains(TypeSet::ARTIFACT));
    assert!(
        subtypes_of(&engine, tokens[0])
            .contains(baylee_core::generated::subtypes::artifact::TREASURE),
        "it is a Treasure"
    );
}
