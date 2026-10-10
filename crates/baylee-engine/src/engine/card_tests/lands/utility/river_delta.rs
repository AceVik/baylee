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
