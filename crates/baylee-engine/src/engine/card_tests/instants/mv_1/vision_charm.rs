//! `cards/instants/mv_1/vision_charm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn vision_charm() -> CardIndex {
    card_index("c56d2c5b-c3e3-4236-bc38-286e250aa4f7")
}

/// `Vision Charm` ({U}, Instant, `Coverage::Partial`) is a modal spell whose
/// first mode makes target player mill four cards and whose third mode phases
/// out target artifact.
#[test]
fn vision_charm_mills_four_or_phases_out_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .battlefield(1, &[sol_ring()])
        .hand(0, &[vision_charm(), vision_charm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p1, sol_ring()).unwrap();
    let gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();

    tap_all_mana(&mut engine, p0);
    let charm1 = in_hand(&engine, p0, vision_charm()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: charm1 })
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        gy_before + 4
    );

    let charm2 = in_hand(&engine, p0, vision_charm()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: charm2 })
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseMode(2)).unwrap();
    aim_at(&mut engine, p0, ring);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(ring)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
}
