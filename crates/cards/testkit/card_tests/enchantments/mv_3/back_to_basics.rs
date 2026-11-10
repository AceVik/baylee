//! `cards/enchantments/mv_3/back_to_basics.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Back to Basics — {2}{U} enchantment: "Nonbasic lands don't untap during
/// their controllers' untap steps." The card is cast for real off three
/// Islands, and the Badlands beside them is the whole proof: it is a land the
/// CR 305.6 shortcut taps for mana like any other, and the difference between
/// the two kinds only shows up one untap step later. The three Islands coming
/// back in that same step are the control — a rule that held *everything*
/// down and a game that never reached CR 502.3 would read identically without
/// them.
#[test]
fn back_to_basics_holds_the_nonbasic_lands_down_while_the_basics_untap() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), badlands()])
        .hand(0, &[back_to_basics()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let nonbasic = on_battlefield(&engine, p0, badlands()).expect("the Badlands is on the table");
    let basics = all_on_battlefield(&engine, p0, island());
    assert_eq!(basics.len(), 3, "three Islands beside it");
    assert!(
        !is_tapped(&engine, nonbasic),
        "a land enters untapped, so it has a {{T}} to spend"
    );

    // {2}{U} out of the board, and every land on it is spent to do it. That
    // the Badlands paid is what makes its *not* untapping below a fact about
    // the printed rule rather than about a land that never moved.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, back_to_basics());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, back_to_basics()).is_some(),
        "the enchantment resolved"
    );
    assert!(is_tapped(&engine, nonbasic), "the Badlands paid for it");
    assert!(
        basics.iter().all(|id| is_tapped(&engine, *id)),
        "and so did every Island"
    );

    // A whole turn cycle, so the untap step that matters is p0's own. The
    // opponent's turn in between is what makes it the *second* untap step of
    // the game rather than the one that already happened before the cast.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        basics.iter().all(|id| !is_tapped(&engine, *id)),
        "the untap step ran: every basic land is back. Without this the \
         assertion below is satisfied by a game that never reached CR 502.3"
    );
    assert!(
        is_tapped(&engine, nonbasic),
        "\"Nonbasic lands don't untap during their controllers' untap \
         steps\" — the Badlands alone stayed down, and the three basic \
         Islands beside it are why that is the rule and not a missing turn"
    );
}
