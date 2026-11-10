//! `cards/creatures/artifacts/mv_3/chrome_cat.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn chrome_cat() -> CardIndex {
    card_index("0e2f8758-7457-4b54-ae61-a9c68cea4a23")
}

/// Chrome Cat — "When this creature enters, scry 1": one card is looked at.
#[test]
fn chrome_cat_scries_one_on_entering() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[chrome_cat()], &[forest(), forest(), forest()]);
    cast_from_hand(&mut engine, p0, chrome_cat());

    let before = answer_scry(&mut engine, 1, true);

    assert_scried(&engine, &before, 1, true);
    let cat = on_battlefield(&engine, p0, chrome_cat()).expect("it resolved");
    assert_eq!(pt(&engine, cat), (3, 2));
}

/// The look may be left on top.
#[test]
fn chrome_cat_may_leave_the_card_on_top() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[chrome_cat()], &[forest(), forest(), forest()]);
    cast_from_hand(&mut engine, p0, chrome_cat());

    let before = answer_scry(&mut engine, 1, false);

    assert_scried(&engine, &before, 1, false);
}
