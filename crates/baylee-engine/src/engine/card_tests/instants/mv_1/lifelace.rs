//! `cards/instants/mv_1/lifelace.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lifelace: "Target spell or permanent becomes green."
#[test]
fn lifelace_turns_its_target_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[oboro_envoy(), forest()])
        .hand(0, &[lifelace()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let envoy = on_battlefield(&engine, p0, oboro_envoy()).expect("seated");
    assert_eq!(
        engine
            .state()
            .object(envoy)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(Color::Blue),
        "blue as printed, before"
    );
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, lifelace());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![envoy],
                players: vec![],
            },
        )
        .expect("its own Envoy is a legal target");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(envoy)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(Color::Green),
        "\"becomes green\""
    );
}
