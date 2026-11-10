//! `cards/creatures/mv_4/abyssal_hunter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn abyssal_hunter() -> CardIndex {
    card_index("b96ccc2f-88e5-4522-924e-76fc76dc3b39")
}

#[test]
fn abyssal_hunter_taps_and_deals_power_damage_to_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[abyssal_hunter(), swamp()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, abyssal_hunter(), 0);
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    let bear_obj = engine.state().object(bear).unwrap();
    assert!(bear_obj.status.contains(Status::TAPPED));
    assert_eq!(bear_obj.damage, 1);
}

#[test]
fn abyssal_hunter_targets_only_creatures_and_needs_to_be_untapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[abyssal_hunter(), swamp(), swamp()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let hunter = on_battlefield(&engine, p0, abyssal_hunter()).unwrap();
    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, abyssal_hunter(), 0);
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert!(options.contains(&bear));
    assert!(player_options.is_empty(), "creatures only, no player");
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, hunter), "the {{T}} cost was paid");
    // The Hunter is tapped now: the ability is not offered again.
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(!legal.abilities.contains(&(hunter, 0)));
    assert_eq!(engine.state().object(bear).unwrap().damage, 1);
}
