//! `cards/creatures/mv_7/thundering_spineback.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn thundering_spineback() -> CardIndex {
    card_index("c9db31d7-00cf-4200-a11f-74cc2d9e843e")
}

/// Raptor Hatchling, a 1/1 Dinosaur, for the anthem to read.
fn raptor_hatchling() -> CardIndex {
    card_index("91638137-06ba-4b21-b088-28d2b4d1d8da")
}

/// Thundering Spineback — a 5/5 for {5}{G}{G}.
#[test]
fn thundering_spineback_costs_seven_and_is_a_five_five() {
    let mut engine = table7(&lots(forest(), 7), &[thundering_spineback()], &[]);
    let spineback = cast_and_resolve(&mut engine, thundering_spineback());
    assert_body(&engine, spineback, (5, 5), KeywordSet::default());
}

/// "Other Dinosaurs you control get +1/+1": my Hatchling is a 2/2, theirs a
/// 1/1, and the Spineback itself stays a 5/5.
#[test]
fn thundering_spineback_pumps_my_other_dinosaurs_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let engine = table7(
        &[thundering_spineback(), raptor_hatchling()],
        &[],
        &[raptor_hatchling()],
    );
    let spineback = on_battlefield(&engine, p0, thundering_spineback()).expect("out");
    let mine = on_battlefield(&engine, p0, raptor_hatchling()).expect("out");
    let theirs = on_battlefield(&engine, p1, raptor_hatchling()).expect("out");
    assert_eq!(pt(&engine, spineback), (5, 5));
    assert_eq!(pt(&engine, mine), (2, 2));
    assert_eq!(pt(&engine, theirs), (1, 1));
}

/// "{5}{G}: Create a 3/3 green Dinosaur creature token with trample": the
/// token is another Dinosaur, so it is 4/4 under the anthem.
#[test]
fn thundering_spineback_makes_a_dinosaur_token() {
    let p0 = PlayerId::new(0);
    let mut board = lots(forest(), 6);
    board.push(thundering_spineback());
    let mut engine = table7(&board, &[], &[]);
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, thundering_spineback(), 1);
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1);
    assert_eq!(pt(&engine, tokens[0]), (4, 4), "3/3 plus the anthem");
    assert!(keywords(&engine, tokens[0]).contains(KeywordSet::TRAMPLE));
}
