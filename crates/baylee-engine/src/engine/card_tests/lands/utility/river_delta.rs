//! Depletion is checked at untap; its counter is removed only in upkeep.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn river_delta_skips_one_untap_then_recovers() {
    use crate::engine::card_tests::legends_kit as kit;
    let mut engine = kit::game(926, &[river_delta()], &[], &[], &[]);
    let land = kit::obj(&engine, kit::P0, river_delta());
    activate(&mut engine, kit::P0, river_delta(), 0);
    engine
        .apply(kit::P0, PlayerAction::ChooseColor(ManaColor::Blue))
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

/// "{T}: Add {U} or {B}.": the colour question names exactly those two, and
/// the one chosen is the one added.
#[test]
fn river_delta_offers_exactly_blue_or_black_and_adds_the_chosen_one() {
    use crate::engine::card_tests::legends_kit as kit;
    let mut engine = kit::game(927, &[river_delta()], &[], &[], &[]);
    activate(&mut engine, kit::P0, river_delta(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected a colour choice, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![ManaColor::Blue, ManaColor::Black]);
    engine
        .apply(kit::P0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();
    assert_eq!(kit::pool_of(&engine, kit::P0, ManaColor::Black), 1);
    assert_eq!(
        kit::pool_total(&engine, kit::P0),
        1,
        "one mana, no other colour"
    );
}
