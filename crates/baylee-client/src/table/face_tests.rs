//! The table's text face (#259): when a face on the table is the one to
//! show, and when it is fitted again.

use super::*;

/// A face fitted before the font arrived is not the face to show once it
/// has. The average's widths are a stand-in and the font's can put the same
/// name on the other number of lines — "Abandoned Outpost" is one line by the
/// average and two by the font (`face::tests`) — so a face kept across the
/// font's arrival would be set in lines its name bar was not made for.
#[test]
fn a_face_fitted_by_the_average_is_fitted_again_when_the_font_arrives() {
    let shown = ShownFace {
        seq: 7,
        measured: false,
        texts: Vec::new(),
    };
    assert!(shown.is(7, false), "an unchanged face was rebuilt");
    assert!(
        !shown.is(7, true),
        "the font arrived and the face kept the average's lines"
    );
    assert!(!shown.is(8, false), "a new snapshot kept the old face");
}
