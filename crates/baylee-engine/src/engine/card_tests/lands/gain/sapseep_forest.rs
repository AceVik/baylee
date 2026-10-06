//! `cards/lands/gain/sapseep_forest.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sapseep Forest: "({T}: Add {G}.)" / "This land enters tapped." / "{G}, {T}: You gain 1 life. Activate only if you control two or more green permanents."
/// Controlling two Llanowar Elves satisfies the condition of controlling two or more green permanents.
/// Paying {G} from a Forest and tapping Sapseep Forest gains 1 life upon resolution.
#[test]
fn sapseep_forest_gains_life_when_controlling_two_green_permanents() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(138, forest())
        .battlefield(
            0,
            &[
                sapseep_forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sapseep = on_battlefield(&engine, p0, sapseep_forest()).expect("Sapseep deployed");
    let life_before = engine.state().players[0].life;

    tap_mana_except(&mut engine, p0, sapseep);
    activate(&mut engine, p0, sapseep_forest(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, life_before + 1);
    assert!(is_tapped(&engine, sapseep));
}
