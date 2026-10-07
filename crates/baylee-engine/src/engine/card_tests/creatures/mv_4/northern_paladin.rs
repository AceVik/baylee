//! `cards/creatures/mv_4/northern_paladin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Northern Paladin — `{W}{W}`, `{T}`: Destroy target black permanent. The
/// menu offers a black permanent and refuses a white one, and the paladin
/// itself taps to pay.
#[test]
fn northern_paladin_destroys_a_black_permanent_and_no_other() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                northern_paladin(),
                plains(),
                plains(),
                plains(),
                scathe_zombies(),
                pearled_unicorn(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let paladin = on_battlefield(&engine, p0, northern_paladin()).expect("seated");
    let black = on_battlefield(&engine, p0, scathe_zombies()).expect("a black permanent");
    let white = on_battlefield(&engine, p0, pearled_unicorn()).expect("a white one, for contrast");

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, northern_paladin(), 0);
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected a target for the destroy, got {:?}",
            engine.pending()
        )
    };
    assert_eq!((min, max), (1, 1), "one target");
    assert!(options.contains(&black), "\"target black permanent\"");
    assert!(
        !options.contains(&white),
        "a white permanent is not a legal target: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![black],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, scathe_zombies()).is_some(),
        "destroyed"
    );
    assert!(
        on_battlefield(&engine, p0, pearled_unicorn()).is_some(),
        "left alone"
    );
    assert!(is_tapped(&engine, paladin), "it paid its own tap");
}
