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

    cast_from_hand(&mut engine, p0, flicker());
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);

    let returned = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    assert!(!is_tapped(&engine, returned));
}
