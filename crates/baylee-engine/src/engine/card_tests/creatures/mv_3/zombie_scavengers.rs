//! `cards/creatures/mv_3/zombie_scavengers.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::LIGHTNING_BOLT;

fn zombie_scavengers() -> CardIndex {
    card_index("8c5298da-6759-444e-8870-6c96d226c940")
}

/// `Zombie Scavengers` ({2}{B}, 3/1 Zombie, Partial) exiles a creature card
/// from its controller's graveyard to regenerate itself.
#[test]
fn zombie_scavengers_exiles_creature_card_from_graveyard_to_regenerate() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                zombie_scavengers(),
                quiet_creature(),
                baylee_core::generated::index::MOUNTAIN,
            ],
        )
        .hand(0, &[LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let scavengers = on_battlefield(&engine, p0, zombie_scavengers()).expect("Scavengers");
    let fodder = on_battlefield(&engine, p0, quiet_creature()).expect("fodder");
    bury(&mut engine, &[fodder]);
    engine.refresh_offer();
    let gy_fodder = in_graveyard(&engine, p0, quiet_creature()).expect("fodder in graveyard");

    activate(&mut engine, p0, zombie_scavengers(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![gy_fodder],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(gy_fodder).unwrap().zone, Zone::Exile);

    cast_from_hand(&mut engine, p0, LIGHTNING_BOLT);
    aim_at(&mut engine, p0, scavengers);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, zombie_scavengers()).is_some());
    assert!(is_tapped(&engine, scavengers));
}
