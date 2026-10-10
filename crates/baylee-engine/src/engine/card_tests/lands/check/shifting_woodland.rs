//! `cards/lands/check/shifting_woodland.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn shifting_woodland_enters_untapped_with_forest_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(305, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[shifting_woodland()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, shifting_woodland());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, shifting_woodland(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn woodland_checks_distinct_types_then_restores_itself_at_cleanup() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[shifting_woodland()])
        .hand(
            0,
            &[
                index::ORNITHOPTER,
                forest(),
                island(),
                swords_to_plowshares(),
            ],
        )
        .hand(1, &[index::HOLY_STRENGTH])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p0, shifting_woodland()).unwrap();
    let model = bury_from_hand(&mut e, p0, index::ORNITHOPTER);
    bury_from_hand(&mut e, p0, forest());
    bury_from_hand(&mut e, p0, island());
    bury_from_hand(&mut e, PlayerId::new(1), index::HOLY_STRENGTH);
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Green, 4);
    e.refresh_offer();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(
        !legal.abilities.contains(&(land, 1)),
        "only three distinct types in our graveyard"
    );
    let instant = bury_from_hand(&mut e, p0, swords_to_plowshares());
    activate(&mut e, p0, shifting_woodland(), 1);
    let offered = aim_at(&mut e, p0, model);
    assert!(!offered.contains(&instant), "permanent cards only");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, land), (0, 2));
    assert!(keywords(&e, land).contains(KeywordSet::FLYING));
    assert_eq!(e.state().object(model).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    pass_until(&mut e, |e| e.state().turn.active != p0);
    assert_eq!(types(&e, land), TypeSet::LAND);
    assert!(!keywords(&e, land).contains(KeywordSet::FLYING));
}
