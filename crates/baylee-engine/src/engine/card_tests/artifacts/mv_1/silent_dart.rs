//! Oracle checklist: pay {4}, tap and sacrifice as activation costs; target a creature only; deal exactly 3 on resolution, with the source gone and the 4/4 surviving marked damage.
#[allow(clippy::wildcard_imports)]
use super::*;

fn silent_dart() -> CardIndex {
    card_index("912bc348-9ba3-4001-90a8-104d2bb17527")
}

#[test]
fn dart_requires_four_mana_and_an_untapped_source() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[silent_dart()])
        .battlefield(1, &[fangren_hunter()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let dart = on_battlefield(&e, p0, silent_dart()).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 3);
    e.refresh_offer();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(
        legal.abilities.iter().all(|(source, _)| *source != dart),
        "three mana cannot pay {{4}}"
    );

    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 1);
    e.state.set_tapped(dart, true);
    e.refresh_offer();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(
        legal.abilities.iter().all(|(source, _)| *source != dart),
        "a tapped Dart cannot pay {{T}}"
    );
}

#[test]
fn dart_pays_costs_before_its_three_damage_resolves_on_a_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[silent_dart()])
        .battlefield(1, &[fangren_hunter(), quiet_artifact()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 4);
    e.refresh_offer();

    let target = on_battlefield(&e, p1, fangren_hunter()).unwrap();
    let artifact = on_battlefield(&e, p1, quiet_artifact()).unwrap();
    activate(&mut e, p0, silent_dart(), 0);
    let Pending::ChooseTargets {
        options, min, max, ..
    } = e.pending().clone()
    else {
        panic!("the Dart asks for its creature target: {:?}", e.pending());
    };
    assert_eq!((min, max), (1, 1));
    assert!(options.contains(&target));
    assert!(
        !options.contains(&artifact),
        "an artifact is not a creature"
    );
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![target],
            players: vec![],
        },
    )
    .unwrap();
    assert_eq!(e.state().players[0].mana_pool.total(), 0, "{{4}} is paid");
    assert!(
        e.state().journal.entries().iter().any(|entry| matches!(
            &entry.event,
            crate::event::GameEvent::AbilityActivated { tap_cost: true, .. }
        )),
        "the activation records a tap cost"
    );
    assert!(
        on_battlefield(&e, p0, silent_dart()).is_none(),
        "sacrifice is paid before resolution"
    );
    assert_eq!(
        e.state().object(target).unwrap().damage,
        0,
        "damage waits for resolution"
    );
    pass_until(&mut e, stack_is_empty);

    assert_eq!(e.state().object(target).unwrap().damage, 3);
    assert_eq!(pt(&e, target), (4, 4));
    assert!(on_battlefield(&e, p1, quiet_artifact()).is_some());
    assert!(in_graveyard(&e, p0, silent_dart()).is_some());
}
