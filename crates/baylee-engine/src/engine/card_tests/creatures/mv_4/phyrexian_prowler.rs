//! `cards/creatures/mv_4/phyrexian_prowler.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn phyrexian_prowler_removes_fade_counter_to_pump_self() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[index::PHYREXIAN_PROWLER])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, index::PHYREXIAN_PROWLER);
    pass_until(&mut engine, stack_is_empty);

    let prowler = on_battlefield(&engine, p0, index::PHYREXIAN_PROWLER).expect("Phyrexian Prowler");
    assert_eq!(
        engine
            .state()
            .object(prowler)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::FADE),
        3
    );
    assert_eq!(pt(&engine, prowler), (3, 3));

    activate(&mut engine, p0, index::PHYREXIAN_PROWLER, 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(prowler)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::FADE),
        2
    );
    assert_eq!(pt(&engine, prowler), (4, 4));
}
