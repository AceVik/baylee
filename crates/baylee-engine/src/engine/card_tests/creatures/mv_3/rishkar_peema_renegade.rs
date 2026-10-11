//! `cards/creatures/mv_3/rishkar_peema_renegade.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn rishkar_peema_renegade_distributes_counters_and_grants_mana_to_ready_creatures() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), young_wolf()])
        .hand(0, &[rishkar_peema_renegade()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, rishkar_peema_renegade());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (0, 2), "up to two target creatures");

    let rishkar =
        on_battlefield(&engine, p0, rishkar_peema_renegade()).expect("rishkar on battlefield");
    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("wolf on battlefield");
    assert!(options.contains(&rishkar));
    assert!(options.contains(&wolf));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![rishkar, wolf],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, rishkar, CounterKind::P1P1), 1);
    assert_eq!(counters_on(&engine, wolf, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, rishkar), (3, 3));
    assert_eq!(pt(&engine, wolf), (2, 2));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.mana_abilities.contains(&rishkar),
        "summoning sickness still matters"
    );
    assert!(legal.mana_abilities.contains(&wolf));
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: wolf })
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
}

#[test]
fn rishkar_grants_for_any_counter_but_only_to_your_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[rishkar_peema_renegade(), grizzly_bears(), quiet_artifact()],
        )
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let own = on_battlefield(&e, p0, grizzly_bears()).unwrap();
    let other = on_battlefield(&e, p1, grizzly_bears()).unwrap();
    for id in [own, other] {
        e.dev_state_mut(p0)
            .unwrap()
            .object_mut(id)
            .unwrap()
            .counters
            .add(CounterKind::Charge, 1);
    }
    e.refresh_offer();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(legal.mana_abilities.contains(&own));
    e.dev_state_mut(p0)
        .unwrap()
        .object_mut(own)
        .unwrap()
        .counters
        .set(CounterKind::Charge, 0);
    e.refresh_offer();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(!legal.mana_abilities.contains(&own));
    pass_until(
        &mut e,
        |e| matches!(e.pending(),Pending::Priority { player,.. } if *player==p1),
    );
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(!legal.mana_abilities.contains(&other));
}
