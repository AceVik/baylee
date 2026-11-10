//! `cards/artifacts/mv_3/floodwater_dam.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn floodwater_dam() -> CardIndex {
    card_index("a3947473-0f57-4937-bb1c-f20969144415")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn floodwater_dam_pays_twice_x_plus_one_to_tap_x_target_lands() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                floodwater_dam(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .battlefield(1, &[mountain(), forest(), plains()])
        .start();

    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dam = on_battlefield(&engine, p0, floodwater_dam()).unwrap();
    let mtn = on_battlefield(&engine, p1, mountain()).unwrap();
    let fst = on_battlefield(&engine, p1, forest()).unwrap();
    let pln = on_battlefield(&engine, p1, plains()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: dam,
                ability_index: 0,
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![mtn, fst],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, dam));
    assert!(
        is_tapped(&engine, mtn) && is_tapped(&engine, fst),
        "Floodwater Dam should tap the X target lands"
    );
    assert!(
        !is_tapped(&engine, pln),
        "untargeted lands should remain untapped"
    );
}

/// "Tap X target lands": exactly X, and lands only. With X = 2 one target, or a creature among the
/// targets, is refused; two lands are tapped and the third land and the creature are not.
#[test]
fn floodwater_dam_takes_exactly_x_lands_and_no_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                floodwater_dam(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .battlefield(1, &[mountain(), forest(), plains(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dam = on_battlefield(&engine, p0, floodwater_dam()).unwrap();
    let mtn = on_battlefield(&engine, p1, mountain()).unwrap();
    let fst = on_battlefield(&engine, p1, forest()).unwrap();
    let pln = on_battlefield(&engine, p1, plains()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: dam,
                ability_index: 0,
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    let choose = |objects: Vec<ObjectId>| PlayerAction::ChooseTargets {
        objects,
        players: vec![],
    };
    assert!(
        engine.apply(p0, choose(vec![mtn])).is_err(),
        "X = 2 needs two targets, not one"
    );
    assert!(
        engine.apply(p0, choose(vec![mtn, bears])).is_err(),
        "a creature is not a land"
    );
    engine.apply(p0, choose(vec![mtn, fst])).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, mtn) && is_tapped(&engine, fst));
    assert!(!is_tapped(&engine, pln), "a land that was not chosen");
    assert!(!is_tapped(&engine, bears), "not a land, not chosen");
}
