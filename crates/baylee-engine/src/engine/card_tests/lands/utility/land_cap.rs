//! Depletion is checked at untap; its counter is removed only in upkeep.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn land_cap_skips_one_untap_then_recovers() {
    use crate::engine::card_tests::legends_kit as kit;
    let mut engine = kit::game(926, &[land_cap()], &[], &[], &[]);
    let land = kit::obj(&engine, kit::P0, land_cap());
    activate(&mut engine, kit::P0, land_cap(), 0);
    engine
        .apply(kit::P0, PlayerAction::ChooseColor(ManaColor::White))
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

#[test]
fn land_cap_offers_exactly_white_or_blue_and_pools_the_one_chosen() {
    use crate::engine::card_tests::legends_kit as kit;
    for (chosen, other) in [
        (ManaColor::White, ManaColor::Blue),
        (ManaColor::Blue, ManaColor::White),
    ] {
        let mut engine = kit::game(927, &[land_cap()], &[], &[], &[]);
        let land = kit::obj(&engine, kit::P0, land_cap());
        activate(&mut engine, kit::P0, land_cap(), 0);
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!("expected a colour choice, got {:?}", engine.pending());
        };
        assert_eq!(
            options.len(),
            2,
            "\"Add {{W}} or {{U}}\": two colours, no more: {options:?}"
        );
        assert!(
            options.contains(&ManaColor::White) && options.contains(&ManaColor::Blue),
            "the two are white and blue: {options:?}"
        );
        engine
            .apply(kit::P0, PlayerAction::ChooseColor(chosen))
            .unwrap();
        assert_eq!(kit::pool_of(&engine, kit::P0, chosen), 1, "the chosen mana");
        assert_eq!(
            kit::pool_of(&engine, kit::P0, other),
            0,
            "and not the other colour"
        );
        assert_eq!(kit::pool_total(&engine, kit::P0), 1, "one mana in all");
        assert_eq!(
            counters_on(&engine, land, counters::DEPLETION),
            1,
            "\"Put a depletion counter on this land\": one"
        );
    }
}
