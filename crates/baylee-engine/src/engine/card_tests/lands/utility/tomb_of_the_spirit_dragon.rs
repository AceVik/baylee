//! `cards/lands/utility/tomb_of_the_spirit_dragon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tomb of the Spirit Dragon: "{T}: Add {C}." / "{2}, {T}: You gain 1 life for each colorless creature you control."
/// With two colorless artifact creatures (Myr Retrievers) under control, the activated ability is paid with two Forests.
/// Upon resolution, the player gains two life, advancing from 20 to 22.
#[test]
fn tomb_of_the_spirit_dragon_gains_life_per_colorless_creature_you_control() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(131, forest())
        .battlefield(
            0,
            &[
                tomb_of_the_spirit_dragon(),
                forest(),
                forest(),
                myr_retriever(),
                myr_retriever(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tomb = on_battlefield(&engine, p0, tomb_of_the_spirit_dragon()).expect("Tomb deployed");
    let life_before = engine.state().players[0].life;

    tap_mana_except(&mut engine, p0, tomb);
    activate(&mut engine, p0, tomb_of_the_spirit_dragon(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        life_before + 2,
        "gained 2 life for 2 colorless creatures"
    );
    assert!(is_tapped(&engine, tomb));
}
