//! `cards/sorceries/mv_2/flicker.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn flicker() -> CardIndex {
    card_index("19ea326f-45ff-4bc0-a957-7d3f22dd8c6f")
}

#[test]
fn flicker_exiles_and_immediately_returns_target_nontoken_permanent() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[flicker()])
        .battlefield(0, &[gray_ogre(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    engine
        .dev_state_mut(p0)
        .unwrap()
        .object_mut(ogre)
        .unwrap()
        .status
        .insert(Status::TAPPED);
    assert!(is_tapped(&engine, ogre));

    engine
        .dev_state_mut(p0)
        .unwrap()
        .object_mut(ogre)
        .unwrap()
        .set_controller(PlayerId::new(1));
    let version = engine.state().object(ogre).unwrap().version;
    let token = crate::resolve::tokens::create_tokens(
        engine.dev_state_mut(p0).unwrap(),
        p0,
        &baylee_cards::tokens::SOLDIER_1_1_WHITE,
        None,
        1,
    )[0];
    engine.refresh_offer();
    cast_from_hand(&mut engine, p0, flicker());
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("nontoken permanent targets")
    };
    assert!(options.contains(&ogre));
    assert!(!options.contains(&token));
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);

    let returned = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    assert!(!is_tapped(&engine, returned));
    assert_ne!(
        engine.state().object(returned).unwrap().version,
        version,
        "it returned as a new incarnation under its owner"
    );
    assert!(
        engine
            .state()
            .object(token)
            .is_some_and(|o| o.zone == Zone::Battlefield)
    );
}
