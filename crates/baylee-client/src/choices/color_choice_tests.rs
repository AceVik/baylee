use super::*;
use baylee_core::color::{Color, ColorSet};
#[test]
fn color_combinations_have_localized_labels_without_casting_costs() {
    let colors = baylee_engine::choice::CastModeKind::Colors(ColorSet::from_slice(&[
        Color::White,
        Color::Black,
        Color::Green,
    ]));
    for lang in [Lang::En, Lang::De] {
        let expected = [Phrase::ColorWhite, Phrase::ColorBlack, Phrase::ColorGreen]
            .map(|p| p.text(lang))
            .join(" + ");
        assert_eq!(
            cast_label(
                colors,
                lang,
                ObjectId::new(0, 0),
                FaceNames {
                    view: None,
                    texts: None
                }
            ),
            expected
        );
    }
}
