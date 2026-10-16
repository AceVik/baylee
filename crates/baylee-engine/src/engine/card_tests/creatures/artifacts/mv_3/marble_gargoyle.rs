//! `cards/creatures/artifacts/mv_3/marble_gargoyle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn marble_gargoyle() -> CardIndex {
    card_index("8a1fd464-760c-43d9-829a-424945d91eb9")
}

/// Marble Gargoyle — flying 2/2; "{W}: This creature gets +0/+1 until end of
/// turn."
#[test]
fn marble_gargoyle_gets_plus_zero_plus_one_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[], &[marble_gargoyle(), plains()]);
    let gargoyle = on_battlefield(&engine, p0, marble_gargoyle()).expect("seated");
    assert_eq!(pt(&engine, gargoyle), (2, 2));
    assert!(keywords(&engine, gargoyle).contains(KeywordSet::FLYING));
    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, marble_gargoyle(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, gargoyle), (2, 3));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0, "{{W}} was the price");
}
