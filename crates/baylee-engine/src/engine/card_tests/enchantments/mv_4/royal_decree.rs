//! `cards/enchantments/mv_4/royal_decree.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::life;

fn royal_decree() -> CardIndex {
    card_index("fca14cbd-5ffc-441f-bccb-7ec22b03acbb")
}

/// Royal Decree ({2}{W}{W} Enchantment):
/// - Cumulative upkeep {W}
/// - "Whenever a Swamp, Mountain, black permanent, or red permanent becomes tapped, this enchantment deals 1 damage to that permanent's controller."
#[test]
fn royal_decree_deals_one_damage_when_swamp_mountain_black_or_red_permanent_becomes_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[royal_decree(), plains()])
        .battlefield(1, &[swamp(), mountain(), forest()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    reach_main_phase(&mut engine, p0);

    // Advance to P1's main phase.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });

    let p1_swamp = on_battlefield(&engine, p1, swamp()).unwrap();
    let p1_mtn = on_battlefield(&engine, p1, mountain()).unwrap();
    let p1_forest = on_battlefield(&engine, p1, forest()).unwrap();
    let life_before = life(&engine, p1);

    // Tapping a Forest does not trigger Royal Decree.
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: p1_forest })
        .unwrap();
    assert!(stack_is_empty(&engine));
    assert_eq!(life(&engine, p1), life_before);

    // Tapping a Swamp triggers Royal Decree for 1 damage to P1.
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: p1_swamp })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p1), life_before - 1);

    // Tapping a Mountain triggers Royal Decree for 1 damage to P1.
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: p1_mtn })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p1), life_before - 2);
}
