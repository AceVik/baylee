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
