//! `cards/creatures/artifacts/mv_2/scorpion_sentinel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn scorpion_sentinel() -> CardIndex {
    card_index("5e53fe9f-9995-47ff-85e9-f09b70d7af15")
}

/// Scorpion Sentinel: a 1/4 that "gets +3/+0 as long as you control seven or
/// more lands". Six lands leave it 1/4; the seventh makes it 4/4.
#[test]
fn scorpion_sentinel_gets_plus_three_power_with_seven_lands() {
    let p0 = PlayerId::new(0);
    let mut board = vec![scorpion_sentinel()];
    board.extend_from_slice(&[forest(); 6]);
    let mut engine = table(&[forest()], &board, &[]);
    let sentinel = on_battlefield(&engine, p0, scorpion_sentinel()).expect("seated");
    assert_eq!(pt(&engine, sentinel), (1, 4), "six lands");
    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(all_on_battlefield(&engine, p0, forest()).len(), 7);
    assert_eq!(pt(&engine, sentinel), (4, 4), "seven lands: +3/+0");
}

/// Lands the opponent controls do not count: six of ours, any number theirs.
#[test]
fn scorpion_sentinel_counts_only_your_lands() {
    let p0 = PlayerId::new(0);
    let mut board = vec![scorpion_sentinel()];
    board.extend_from_slice(&[forest(); 6]);
    let engine = table(&[], &board, &[forest(); 8]);
    let sentinel = on_battlefield(&engine, p0, scorpion_sentinel()).expect("seated");
    assert_eq!(pt(&engine, sentinel), (1, 4));
}

/// The boost is a static effect, not a one-time one: it attacks for four with
/// seven lands.
#[test]
fn scorpion_sentinel_attacks_for_four_with_seven_lands() {
    let p0 = PlayerId::new(0);
    let mut board = vec![scorpion_sentinel()];
    board.extend_from_slice(&[forest(); 7]);
    let mut engine = table(&[], &board, &[]);
    let sentinel = on_battlefield(&engine, p0, scorpion_sentinel()).expect("seated");
    swing_unblocked(&mut engine, sentinel);
    assert_eq!(life_of(&engine, 1), 16);
}
