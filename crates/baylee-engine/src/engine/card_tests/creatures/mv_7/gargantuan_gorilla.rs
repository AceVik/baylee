//! `cards/creatures/mv_7/gargantuan_gorilla.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn gargantuan_gorilla() -> CardIndex {
    card_index("8314bdf5-1539-4215-949e-f46584cb43bb")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

#[test]
fn gargantuan_gorilla_fights_with_tap_and_upkeep_sacrifices_forest_or_deals_seven() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gargantuan_gorilla(), forest()])
        .battlefield(1, &[hill_giant()])
        .start();

    keep_mulligans(&mut engine);
    // On turn 1 upkeep, sacrifice the Forest to keep Gargantuan Gorilla.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let fst = on_battlefield(&engine, p0, forest()).unwrap();
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![fst] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    let gorilla = on_battlefield(&engine, p0, gargantuan_gorilla()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();

    // Activate {T}: mutual power damage with Hill Giant (3/3).
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: gorilla,
                ability_index: 1,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, hill_giant()).is_none(),
        "Gargantuan Gorilla should deal 7 damage to Hill Giant, destroying it"
    );
    assert_eq!(
        engine.state().object(gorilla).unwrap().damage,
        3,
        "Hill Giant should deal 3 damage back to Gargantuan Gorilla"
    );

    // On p0's next upkeep with no Forests, Gargantuan Gorilla sacrifices itself and deals 7 damage to p0.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, gargantuan_gorilla()).is_some(),
        "\"your upkeep\": the opponent's upkeep costs the Gorilla nothing"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and 7 damage is not dealt yet"
    );
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, gargantuan_gorilla()).is_none(),
        "Gargantuan Gorilla should be sacrificed when no Forest is sacrificed"
    );
    assert_eq!(
        engine.state().players[0].life,
        13,
        "Gargantuan Gorilla should deal 7 damage to p0 when not fed a Forest"
    );
}

/// Player 0's Gorilla stands with a Forest and a Mountain against player 1's Hill Giant, stopped at
/// the first upkeep question ("you may sacrifice a Forest").
fn gorilla_at_its_upkeep() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gargantuan_gorilla(), forest(), mountain()])
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
}

fn finish_the_upkeep(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, |e| {
        e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
}

/// "You may sacrifice a Forest": only the Forest is on the menu (not the Mountain, not the Gorilla),
/// and paying it keeps the Gorilla, costs no life and leaves the Mountain alone.
#[test]
fn gargantuan_gorilla_upkeep_offers_only_forests_and_paying_keeps_it() {
    let p0 = PlayerId::new(0);
    let mut engine = gorilla_at_its_upkeep();
    let forest_id = on_battlefield(&engine, p0, forest()).unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the sacrifice question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options, vec![forest_id], "a Forest is the whole menu");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![forest_id],
            },
        )
        .unwrap();
    finish_the_upkeep(&mut engine);

    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "the Forest was sacrificed"
    );
    assert!(on_battlefield(&engine, p0, mountain()).is_some());
    assert!(on_battlefield(&engine, p0, gargantuan_gorilla()).is_some());
    assert_eq!(engine.state().players[0].life, 20);
}

/// "If you don't sacrifice a Forest, sacrifice this creature and it deals 7 damage to you": naming
/// nothing declines, the Forest stays, the Gorilla goes and its controller takes 7.
#[test]
fn gargantuan_gorilla_declining_the_forest_sacrifices_it_and_hurts_its_controller() {
    let p0 = PlayerId::new(0);
    let mut engine = gorilla_at_its_upkeep();
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    finish_the_upkeep(&mut engine);

    assert!(on_battlefield(&engine, p0, gargantuan_gorilla()).is_none());
    assert!(in_graveyard(&engine, p0, gargantuan_gorilla()).is_some());
    assert!(
        on_battlefield(&engine, p0, forest()).is_some(),
        "no Forest was paid"
    );
    assert_eq!(engine.state().players[0].life, 13);
    assert_eq!(
        engine.state().players[1].life,
        20,
        "only its controller is hurt"
    );
}

/// "Another target creature": the Gorilla itself and a land are refused, the fight taps it, and a
/// tapped Gorilla cannot fight again.
#[test]
fn gargantuan_gorilla_fights_only_another_creature_and_only_once_untapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = gorilla_at_its_upkeep();
    let forest_id = on_battlefield(&engine, p0, forest()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![forest_id],
            },
        )
        .unwrap();
    finish_the_upkeep(&mut engine);

    let gorilla = on_battlefield(&engine, p0, gargantuan_gorilla()).unwrap();
    let mountain_id = on_battlefield(&engine, p0, mountain()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: gorilla,
                ability_index: 1,
            },
        )
        .unwrap();
    for illegal in [gorilla, mountain_id] {
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![illegal],
                        players: vec![],
                    },
                )
                .is_err(),
            "{illegal:?} is no other creature"
        );
    }
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, hill_giant()).is_none());
    assert_eq!(engine.state().object(gorilla).unwrap().damage, 3);
    assert_eq!(engine.state().object(mountain_id).unwrap().damage, 0);
    assert!(is_tapped(&engine, gorilla), "{{T}} is part of the cost");
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: gorilla,
                    ability_index: 1,
                },
            )
            .is_err(),
        "a tapped Gorilla cannot fight again"
    );
}
