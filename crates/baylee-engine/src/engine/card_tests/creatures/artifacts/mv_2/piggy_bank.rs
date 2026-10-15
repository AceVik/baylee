//! `cards/creatures/artifacts/mv_2/piggy_bank.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn piggy_bank() -> CardIndex {
    card_index("a13f4095-300b-434f-aa9a-3608aaae5163")
}

/// Piggy Bank: a 3/2 `{1}{R}` artifact creature; "when this creature dies,
/// create a Treasure token". No token while it lives, exactly one after.
#[test]
fn piggy_bank_leaves_a_treasure_when_it_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[piggy_bank()], &[]);
    let bank = on_battlefield(&engine, p0, piggy_bank()).expect("seated");
    assert_eq!(pt(&engine, bank), (3, 2));
    assert!(tokens_of(&engine, p0).is_empty());
    kill(&mut engine, bank);
    assert!(in_graveyard(&engine, p0, piggy_bank()).is_some());
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1);
    let chars = engine.state().object(tokens[0]).unwrap().characteristics();
    assert_eq!(engine.state().names.get(chars.name), "Treasure");
    assert!(chars.types.contains(TypeSet::ARTIFACT));
}

/// Cast for `{1}{R}` and attacking unblocked it deals three.
#[test]
fn piggy_bank_is_cast_for_one_and_red_and_hits_for_three() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[piggy_bank()], &[mountain(), forest()], &[]);
    cast_and_land(&mut engine, piggy_bank());
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p0, piggy_bank()).is_none());

    let mut engine = table(&[], &[piggy_bank()], &[]);
    let bank = on_battlefield(&engine, p0, piggy_bank()).expect("seated");
    swing_unblocked(&mut engine, bank);
    assert_eq!(life_of(&engine, 1), 17);
}
