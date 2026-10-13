//! Depletion is checked at untap; its counter is removed only in upkeep.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn lava_tubes_skips_one_untap_then_recovers() {
    use crate::engine::card_tests::legends_kit as kit;
    let mut engine = kit::game(926, &[lava_tubes()], &[], &[], &[]);
    let land = kit::obj(&engine, kit::P0, lava_tubes());
    activate(&mut engine, kit::P0, lava_tubes(), 0);
    engine
        .apply(kit::P0, PlayerAction::ChooseColor(ManaColor::Black))
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
fn lava_tubes_offers_exactly_black_or_red_and_pools_the_one_chosen() {
    use crate::engine::card_tests::legends_kit as kit;
    for (chosen, other) in [
        (ManaColor::Black, ManaColor::Red),
        (ManaColor::Red, ManaColor::Black),
    ] {
        let mut engine = kit::game(927, &[lava_tubes()], &[], &[], &[]);
        let land = kit::obj(&engine, kit::P0, lava_tubes());
        activate(&mut engine, kit::P0, lava_tubes(), 0);
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!("expected a colour choice, got {:?}", engine.pending());
        };
        assert_eq!(
            options.len(),
            2,
            "\"Add {{B}} or {{R}}\": two colours, no more: {options:?}"
        );
        assert!(
            options.contains(&ManaColor::Black) && options.contains(&ManaColor::Red),
            "the two are black and red: {options:?}"
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
