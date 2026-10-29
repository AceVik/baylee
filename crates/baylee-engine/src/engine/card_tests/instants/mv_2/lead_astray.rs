//! `cards/instants/mv_2/lead_astray.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn lead_astray_taps_up_to_two_target_creatures() {
    let lead_astray = card_index("9e8e5ad4-05a3-4757-8797-c37bf49506d5");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .battlefield(1, &[grizzly_bears(), gray_ogre(), llanowar_elves()])
        .hand(0, &[lead_astray])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    let elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    cast_from_hand(&mut engine, p0, lead_astray);
    let Pending::ChooseTargets { min, max, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets");
    };
    assert_eq!((min, max), (0, 2));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear, ogre],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, bear));
    assert!(is_tapped(&engine, ogre));
    assert!(!is_tapped(&engine, elves));
}
