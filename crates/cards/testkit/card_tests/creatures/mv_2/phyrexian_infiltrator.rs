//! `cards/creatures/mv_2/phyrexian_infiltrator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn phyrexian_infiltrator() -> CardIndex {
    card_index("f444a0f8-32d8-4748-af58-51a1a8ed72b4")
}

fn table() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                phyrexian_infiltrator(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, PlayerId::new(0));
    engine
}

fn to_my_next_turn(engine: &mut Engine<RegistryLookup>) {
    let p0 = PlayerId::new(0);
    let now = engine.state().turn.number;
    pass_until(engine, |e| {
        e.state().turn.number > now
            && e.state().turn.active == p0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { .. })
    });
}

#[test]
fn infiltration_exchanges_both_creatures_indefinitely() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table();
    let infiltrator =
        on_battlefield(&engine, p0, phyrexian_infiltrator()).expect("the Infiltrator is out");
    let target = on_battlefield(&engine, p1, quiet_creature()).expect("the target is out");

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, phyrexian_infiltrator(), 0);
    let options = aim_at(&mut engine, p0, target);
    assert_eq!(options.len(), 2, "either creature is a legal target");
    assert!(options.contains(&infiltrator) && options.contains(&target));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the four Islands pay exactly {{2}}{{U}}{{U}}"
    );
    assert!(
        !stack_is_empty(&engine),
        "the activated ability is targeted"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(infiltrator).unwrap().controller, p1);
    assert_eq!(engine.state().object(target).unwrap().controller, p0);
    assert_eq!(engine.state().object(infiltrator).unwrap().owner, p0);
    assert_eq!(engine.state().object(target).unwrap().owner, p1);

    to_my_next_turn(&mut engine);
    assert_eq!(
        engine.state().object(infiltrator).unwrap().controller,
        p1,
        "the exchange does not end at cleanup"
    );
    assert_eq!(engine.state().object(target).unwrap().controller, p0);
}

#[test]
fn infiltration_does_nothing_when_its_target_becomes_illegal() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table();
    let infiltrator =
        on_battlefield(&engine, p0, phyrexian_infiltrator()).expect("the Infiltrator is out");
    let target = on_battlefield(&engine, p1, quiet_creature()).expect("the target is out");

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, phyrexian_infiltrator(), 0);
    aim_at(&mut engine, p0, target);
    engine
        .dev_state_mut(p0)
        .expect("the harness accepts development commands")
        .move_object(
            target,
            ZoneLocation::Hand(p1),
            baylee_engine::zone::ZonePosition::Top,
            baylee_engine::event::Cause::Effect,
        )
        .expect("the target leaves before resolution");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(infiltrator).unwrap().controller,
        p0,
        "an ability whose only target is illegal does not resolve"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p1))
            .contains(&target),
        "the departed target stays in its owner's hand"
    );
}
