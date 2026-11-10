//! `cards/enchantments/mv_5/tornado.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn tornado() -> CardIndex {
    card_index("dd283473-6423-4a1f-b5a3-77234cdce643")
}

/// Tornado ({4}{G} Enchantment):
/// - Cumulative upkeep {G}
/// - "{2}{G}, Pay 3 life for each velocity counter on this enchantment: Destroy target permanent and put a velocity counter on this enchantment. Activate only once each turn."
#[test]
fn tornado_destroys_target_permanent_once_per_turn_and_adds_velocity_counter() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                tornado(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[grizzly_bears(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);

    // Upkeep: pay cumulative upkeep {G} for the first age counter.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    reach_main_phase(&mut engine, p0);

    let torn = on_battlefield(&engine, p0, tornado()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    engine.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Green, 6);
    engine.refresh_offer();

    activate(&mut engine, p0, tornado(), 1);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert_eq!(
        engine
            .state()
            .object(torn)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::VELOCITY),
        1
    );

    // Once-per-turn limit prevents a second activation on the same turn.
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority");
    };
    assert!(!legal.abilities.contains(&(torn, 1)));

    // With a velocity counter on Tornado the next activation would cost 3
    // life, and that payment is not played: on P0's next turn (second
    // cumulative upkeep, {G}{G}) the ability is still not offered.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.number > 1
            && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    reach_main_phase(&mut engine, p0);
    let object = engine.state().object(torn).expect("still in play");
    assert_eq!(object.zone, Zone::Battlefield);
    assert_eq!(
        object.counters.get(baylee_cards_dsl::counters::AGE),
        2,
        "a second age counter"
    );
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority");
    };
    assert!(
        !legal.abilities.contains(&(torn, 1)),
        "not offered again with a velocity counter on it"
    );
    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_some());
}
