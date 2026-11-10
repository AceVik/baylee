//! `cards/enchantments/mv_1/font_of_fertility.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn font_of_fertility() -> CardIndex {
    card_index("7fb6f960-38f6-45fa-bfec-d43fc174daa7")
}

/// "{1}{G}, Sacrifice this enchantment: Search your library for a basic land
/// card, put it onto the battlefield tapped, then shuffle."
#[test]
fn font_of_fertility_fetches_a_tapped_basic_land() {
    let mut e = table(7161, &[], &[font_of_fertility()], &[]);
    activate(&mut e, P0, font_of_fertility(), 0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    drive(&mut e, &[], &[], &[], true);
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, P0, font_of_fertility()).is_some());
    let forest = on_battlefield(&e, P0, forest()).expect("a Forest came from the library");
    assert!(is_tapped(&e, forest), "it enters tapped");
}
