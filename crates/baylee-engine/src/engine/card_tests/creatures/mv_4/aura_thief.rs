//! `cards/creatures/mv_4/aura_thief.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn aura_thief() -> CardIndex {
    card_index("cf0936ea-2741-43a4-ad6a-a1258b54945b")
}

#[test]
fn aura_thief_gains_control_of_all_enchantments_when_it_dies() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[aura_thief(), forest()])
        .hand(0, &[rancor()])
        .battlefield(1, &[sylvan_library(), gray_ogre(), sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let thief = on_battlefield(&engine, p0, aura_thief()).unwrap();
    assert!(on_battlefield(&engine, p1, sylvan_library()).is_some());

    let host = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, rancor());
    aim_at(&mut engine, p0, host);
    pass_until(&mut engine, stack_is_empty);
    let aura = on_battlefield(&engine, p0, rancor()).unwrap();
    engine
        .dev_state_mut(p0)
        .unwrap()
        .object_mut(aura)
        .unwrap()
        .set_controller(p1);
    engine.refresh_offer();
    kill(&mut engine, thief);

    assert!(on_battlefield(&engine, p0, sylvan_library()).is_some());
    assert!(
        on_battlefield(&engine, p0, rancor()).is_some(),
        "Auras are enchantments too"
    );
    assert_eq!(
        engine.state().object(aura).unwrap().attached_to,
        Some(host),
        "control change does not move the Aura"
    );
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_some());
    assert!(on_battlefield(&engine, p1, sol_ring()).is_some());
}
