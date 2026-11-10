//! `cards/creatures/artifacts/mv_3/opaline_unicorn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn opaline_unicorn() -> CardIndex {
    card_index("ca5ca2c9-2ddd-4e70-b738-8de61aa82570")
}

/// Opaline Unicorn — "{T}: Add one mana of any color".
#[test]
fn opaline_unicorn_taps_for_one_mana_of_the_color_named() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[], &[opaline_unicorn()]);
    let unicorn = on_battlefield(&engine, p0, opaline_unicorn()).expect("seated");
    assert_eq!(pt(&engine, unicorn), (1, 2));

    tap_for_any_color(&mut engine, unicorn, ManaColor::White);
    assert!(stack_is_empty(&engine));
}
