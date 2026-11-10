//! `cards/artifacts/mv_2/sword_of_the_chosen.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Sword of the Chosen` prints `{{T}}: Target legendary creature gets +2/+2 until end of turn.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Sword of the Chosen`, `Katara, the Fearless`, and a `llanowar_elves()`.
/// Tapping the sword targets only the legendary creature, filtering out non-legendary creatures,
/// and pumps `Katara, the Fearless` from (3, 3) to (5, 5) until end of turn upon resolution.
#[test]
fn sword_of_the_chosen_pumps_target_legendary_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                sword_of_the_chosen(),
                katara_the_fearless(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sword = on_battlefield(&engine, p0, sword_of_the_chosen()).expect("sword on battlefield");
    let katara = on_battlefield(&engine, p0, katara_the_fearless()).expect("katara on battlefield");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("elves on battlefield");
    assert_eq!(pt(&engine, katara), (3, 3));

    activate(&mut engine, p0, sword_of_the_chosen(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&katara),
        "legendary creature is a legal target"
    );
    assert!(
        !options.contains(&elves),
        "non-legendary creature is excluded"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![katara],
            },
        )
        .expect("targeted katara");

    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, sword), "`Sword of the Chosen` is tapped");
    assert_eq!(
        pt(&engine, katara),
        (5, 5),
        "legendary creature received +2/+2"
    );
}
