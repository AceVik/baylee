//! `cards/creatures/artifacts/mv_3/automatic_librarian.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn automatic_librarian() -> CardIndex {
    card_index("eb570331-35da-44b8-b711-ccf7444d8e39")
}

/// Automatic Librarian — "When this creature enters, scry 2": the look is
/// the top two cards, and sending both to the bottom moves them there.
#[test]
fn automatic_librarian_scries_two_on_entering_and_can_bottom_both() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[automatic_librarian()], &[forest(), forest(), forest()]);
    cast_from_hand(&mut engine, p0, automatic_librarian());

    let before = answer_scry(&mut engine, 2, true);

    assert_scried(&engine, &before, 2, true);
    let body = on_battlefield(&engine, p0, automatic_librarian()).expect("it resolved");
    assert_eq!(pt(&engine, body), (3, 2));
}

/// Keeping both on top leaves the library as it was.
#[test]
fn automatic_librarian_can_keep_both_on_top() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[automatic_librarian()], &[forest(), forest(), forest()]);
    cast_from_hand(&mut engine, p0, automatic_librarian());

    let before = answer_scry(&mut engine, 2, false);

    assert_scried(&engine, &before, 2, false);
}
