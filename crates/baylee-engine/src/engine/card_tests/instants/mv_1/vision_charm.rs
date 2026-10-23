//! `cards/instants/mv_1/vision_charm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn vision_charm() -> CardIndex {
    card_index("c56d2c5b-c3e3-4236-bc38-286e250aa4f7")
}

/// Mode 0, "Target player mills four cards": the chosen player mills four, the
/// caster mills none.
#[test]
fn vision_charm_mills_the_chosen_player_four_cards() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .battlefield(1, &[sol_ring()])
        .hand(0, &[vision_charm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p1, sol_ring()).unwrap();
    let gy1_before = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();
    let gy0_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();

    cast_from_hand(&mut engine, p0, vision_charm());
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        gy1_before + 4
    );
    // The caster's own graveyard only gained the resolved charm.
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        gy0_before + 1
    );
    // The mill mode does not touch the artifact.
    assert!(
        !engine
            .state()
            .object(ring)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
}

/// Mode 1, "Target artifact phases out": only artifacts are on the menu (a
/// creature is not), the artifact phases out and phases in again before its
/// controller untaps.
#[test]
fn vision_charm_phases_out_only_an_artifact_until_its_controllers_untap() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .battlefield(1, &[sol_ring(), llanowar_elves()])
        .hand(0, &[vision_charm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p1, sol_ring()).unwrap();
    let elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    cast_from_hand(&mut engine, p0, vision_charm());
    engine.apply(p0, PlayerAction::ChooseMode(1)).unwrap();
    let menu = aim_at(&mut engine, p0, ring);
    assert!(menu.contains(&ring), "the artifact is a legal target");
    assert!(!menu.contains(&elves), "a creature is not an artifact");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(ring)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
    assert!(
        !engine
            .state()
            .object(elves)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );

    // It phases in before its controller untaps.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !engine
            .state()
            .object(ring)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT),
        "phased in again at its controller's untap step"
    );
}
