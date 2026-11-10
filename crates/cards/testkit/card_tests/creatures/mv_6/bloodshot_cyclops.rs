//! `cards/creatures/mv_6/bloodshot_cyclops.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn bloodshot_cyclops() -> CardIndex {
    card_index("84d47913-53ac-4d4d-bac3-bc0306d43e12")
}

#[test]
fn bloodshot_cyclops_sacrifices_creature_to_deal_its_power_to_any_target() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[bloodshot_cyclops(), earth_elemental(), sol_ring()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elem = on_battlefield(&engine, p0, earth_elemental()).unwrap();
    activate(&mut engine, p0, bloodshot_cyclops(), 0);
    if let Pending::ChooseCards { options, .. } = engine.pending() {
        let nonmatching = on_battlefield(&engine, p0, sol_ring()).unwrap();
        let opponent = on_battlefield(&engine, PlayerId::new(1), llanowar_elves()).unwrap();
        assert!(!options.contains(&nonmatching));
        assert!(!options.contains(&opponent));
    }
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![elem],
                },
            )
            .unwrap();
    }
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p1],
                },
            )
            .unwrap();
    }
    if let Pending::ChooseCards { options, .. } = engine.pending() {
        let nonmatching = on_battlefield(&engine, p0, sol_ring()).unwrap();
        let opponent = on_battlefield(&engine, PlayerId::new(1), llanowar_elves()).unwrap();
        assert!(!options.contains(&nonmatching));
        assert!(!options.contains(&opponent));
    }
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![elem],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, earth_elemental()).is_some());
    assert_eq!(engine.state().players[1].life, 16);
}
