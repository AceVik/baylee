//! `cards/creatures/mv_5/staunch_defenders.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Staunch Defenders is `{3}{W}{W}` for a 3/4 Human Soldier whose entire
/// printed text is one enters-trigger: "When this creature enters, you gain 4
/// life." Neither half of that is visible by reading the card file, so both are
/// read off one cast on a board built to make them exact — five Plains pay the
/// `{3}{W}{W}` down to an empty pool, the body is the printed 3/4, and the four
/// life lands on the seat that cast it while the seat across the table stays at
/// twenty, which a card that had gained for the whole table could not tell
/// apart.
#[test]
fn staunch_defenders_arrives_as_a_three_four_and_gains_its_controller_four_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(); 5])
        .hand(0, &[staunch_defenders()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "a life total for the trigger to move"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and one across the table that the trigger must leave alone"
    );

    // Five Plains are the whole cost, and `cast_from_hand` taps them before the
    // cast rather than claiming the spell is castable off untapped lands.
    cast_from_hand(&mut engine, p0, staunch_defenders());
    // The permanent arrives with its trigger still on the stack, so the walk is
    // past the stack rather than merely up to the battlefield: a reading taken
    // now would count the body before the life was gained.
    pass_until(&mut engine, stack_is_empty);

    let soldier = on_battlefield(&engine, p0, staunch_defenders()).expect("the Soldier resolved");
    assert_eq!(
        pt(&engine, soldier),
        (3, 4),
        "the body the card prints, and not a 3/4 from somewhere else on the board"
    );
    assert_eq!(
        engine.state().players[0].life,
        24,
        "\"you gain 4 life\" — four, once, to the seat that cast it"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the life belongs to the controller and never across the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "five Plains paid the {{3}}{{W}}{{W}} to the last mana"
    );
}
