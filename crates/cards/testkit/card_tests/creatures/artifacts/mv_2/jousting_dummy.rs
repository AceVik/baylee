//! `cards/creatures/artifacts/mv_2/jousting_dummy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn jousting_dummy() -> CardIndex {
    card_index("96eea161-8af0-452f-b992-576efdf58d88")
}

/// Jousting Dummy: a 2/1 with "{3}: This creature gets +1/+0 until end of
/// turn". Three mana pump it once; six pump it twice; two do not suffice.
#[test]
fn jousting_dummy_gets_plus_one_power_per_three_mana() {
    let p0 = PlayerId::new(0);
    let mut board = vec![jousting_dummy()];
    board.extend_from_slice(&[forest(); 6]);
    let mut engine = table(&[], &board, &[]);
    let dummy = on_battlefield(&engine, p0, jousting_dummy()).expect("seated");
    assert_eq!(pt(&engine, dummy), (2, 1));
    let forests = all_on_battlefield(&engine, p0, forest());
    tap_mana_where(&mut engine, p0, |id| forests[..2].contains(&id));
    assert!(
        !priority_offer(&engine).abilities.contains(&(dummy, 0)),
        "two mana do not pay {{3}}"
    );
    tap_mana_where(&mut engine, p0, |id| forests[2..3].contains(&id));
    activate(&mut engine, p0, jousting_dummy(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, dummy), (3, 1), "+1/+0");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    tap_mana_where(&mut engine, p0, |id| forests[3..].contains(&id));
    activate(&mut engine, p0, jousting_dummy(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, dummy), (4, 1), "stacking");
    swing_unblocked(&mut engine, dummy);
    assert_eq!(life_of(&engine, 1), 16);
}

/// The pump ends with the turn.
#[test]
fn jousting_dummy_pump_ends_with_the_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[jousting_dummy(), forest(), forest(), forest()], &[]);
    let dummy = on_battlefield(&engine, p0, jousting_dummy()).expect("seated");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, jousting_dummy(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, dummy), (3, 1));
    to_my_next_turn(&mut engine);
    assert_eq!(pt(&engine, dummy), (2, 1));
}
