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
        lines: 1,
        texts: Vec::new(),
    };
    assert!(shown.is(7, false), "an unchanged face was rebuilt");
    assert!(
        !shown.is(7, true),
        "the font arrived and the face kept the average's lines"
    );
    assert!(!shown.is(8, false), "a new snapshot kept the old face");
}

/// The shipped Regular cut, read the way the client reads it.
fn regular() -> Font {
    let path = format!(
        "{}/assets/fonts/AlegreyaSans-Regular.ttf",
        env!("CARGO_MANIFEST_DIR")
    );
    Font::from_bytes(std::fs::read(path).expect("the bundled Regular"))
}

/// A name the average puts on one line and the font on two.
fn outpost() -> CardFace {
    CardFace {
        name: "Abandoned Outpost".to_owned(),
        cost: Vec::new(),
        type_line: "Land".to_owned(),
        body: Vec::new(),
        stats: None,
        colors: ColorSet::EMPTY,
        types: TypeSet::LAND,
        subtypes: SubtypeSet::EMPTY,
        text_pending: false,
    }
}

/// When the font arrives mid-game, a face fitted by the average is fitted
/// again — and the material is re-keyed with it, so the name's lines and the
/// name bar the material draws never disagree. "Abandoned Outpost" is one
/// line by the average and two by the font: kept on its old material, it
/// would be two lines of text under a one-line bar.
#[test]
fn the_font_s_arrival_re_keys_the_material_with_the_text() {
    let font = regular();
    let guessed = face::Widths::of(None);
    let measured = face::Widths::of(Some(&font));
    let look = |now: &FaceNow| {
        face_look(
            None,
            now.lines(),
            FinishTreatment::Plain,
            0,
            cardplate::Corner::default(),
        )
    };
    let two_lines = |look: CardLook| {
        look.face >> textface::FACE_NAME_SHIFT & 0xff == u32::from(textface::Depths::table(2).name)
    };

    // Before the font: fitted by the average, on one line, under a one-line bar.
    let first = face_now(None, 7, &guessed, || Some(outpost()));
    let FaceNow::Fitted(fitted) = &first else {
        panic!("a card with no face was given none");
    };
    assert_eq!(fitted.1.lines(), 1);
    let before = look(&first);
    assert!(!two_lines(before));

    // The same snapshot once the font is there: fitted again, onto two
    // lines, and the look goes with it.
    let shown = ShownFace {
        seq: 7,
        measured: false,
        lines: first.lines(),
        texts: Vec::new(),
    };
    let second = face_now(Some(&shown), 7, &measured, || Some(outpost()));
    let FaceNow::Fitted(refitted) = &second else {
        panic!("the font arrived and the face was kept");
    };
    assert_eq!(refitted.1.lines(), 2);
    let after = look(&second);
    assert!(
        two_lines(after),
        "two lines of name under the material's one-line bar"
    );
    assert_ne!(before, after);

    // And kept from then on, text and look alike, without being rebuilt.
    let shown = ShownFace {
        seq: 7,
        measured: true,
        lines: second.lines(),
        texts: Vec::new(),
    };
    let third = face_now(Some(&shown), 7, &measured, || {
        panic!("a current face was built again")
    });
    assert!(matches!(third, FaceNow::Kept(2)));
    assert_eq!(look(&third), after);
}
