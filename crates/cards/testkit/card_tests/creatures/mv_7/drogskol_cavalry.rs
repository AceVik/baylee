//! `cards/creatures/mv_7/drogskol_cavalry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn drogskol_cavalry() -> CardIndex {
    card_index("a8bac5a8-5a60-4f7a-a4d6-cb922da92472")
}

/// Drogskol Cavalry — a 4/4 flier for {5}{W}{W}. "Whenever another Spirit you
/// control enters" does not read the Cavalry itself (a Spirit Knight) coming
/// in: its controller's life stays at 20.
#[test]
fn drogskol_cavalry_costs_seven_and_gains_nothing_for_itself() {
    let mut engine = table7(&lots(plains(), 7), &[drogskol_cavalry()], &[]);
    let cavalry = cast_and_resolve(&mut engine, drogskol_cavalry());
    assert_body(&engine, cavalry, (4, 4), KeywordSet::FLYING);
    assert_eq!(life_of(&engine, 0), 20, "\"another\" Spirit");
}

/// "{3}{W}: Create a 1/1 white Spirit creature token with flying." The token
/// is another Spirit entering under its controller, so the trigger gains 2.
#[test]
fn drogskol_cavalry_makes_a_spirit_and_gains_two() {
    let p0 = PlayerId::new(0);
    let mut board = lots(plains(), 4);
    board.push(drogskol_cavalry());
    let mut engine = table7(&board, &[], &[]);
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, drogskol_cavalry(), 1);
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Spirit token");
    assert_body(&engine, tokens[0], (1, 1), KeywordSet::FLYING);
    assert_eq!(life_of(&engine, 0), 22, "the Spirit entering gained 2");
}
