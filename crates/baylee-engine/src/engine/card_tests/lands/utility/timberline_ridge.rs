//! Depletion is checked at untap; its counter is removed only in upkeep.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn timberline_ridge_skips_one_untap_then_recovers() {
    use crate::engine::card_tests::legends_kit as kit;
    let mut engine = kit::game(926, &[timberline_ridge()], &[], &[], &[]);
    let land = kit::obj(&engine, kit::P0, timberline_ridge());
    activate(&mut engine, kit::P0, timberline_ridge(), 0);
    engine
        .apply(kit::P0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    assert_eq!(counters_on(&engine, land, counters::DEPLETION), 1);
    assert!(is_tapped(&engine, land));
    cross_into_the_next_own_main(&mut engine, kit::P0);
    assert_eq!(counters_on(&engine, land, counters::DEPLETION), 0);
    assert!(
        is_tapped(&engine, land),
        "the upkeep trigger comes after untap"
    );
    cross_into_the_next_own_main(&mut engine, kit::P0);
    assert!(!is_tapped(&engine, land));
}

/// "{T}: Add {R} or {G}.": the colour question names exactly those two, and
/// the one chosen is the one added.
#[test]
fn timberline_ridge_offers_exactly_red_or_green_and_adds_the_chosen_one() {
    use crate::engine::card_tests::legends_kit as kit;
    let mut engine = kit::game(927, &[timberline_ridge()], &[], &[], &[]);
    activate(&mut engine, kit::P0, timberline_ridge(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected a colour choice, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![ManaColor::Red, ManaColor::Green]);
    engine
        .apply(kit::P0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    assert_eq!(kit::pool_of(&engine, kit::P0, ManaColor::Green), 1);
    assert_eq!(
        kit::pool_total(&engine, kit::P0),
        1,
        "one mana, no other colour"
    );
}
