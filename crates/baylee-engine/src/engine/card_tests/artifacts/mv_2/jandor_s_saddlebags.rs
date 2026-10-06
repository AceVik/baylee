//! `cards/artifacts/mv_2/jandor_s_saddlebags.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Jandor's Saddlebags` prints `{{3}}, {{T}}: Untap target creature.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Jandor's Saddlebags`, a `llanowar_elves()`, and three copies of `forest()`.
/// Tapping the other mana sources leaves the elf creature tapped while floating mana.
/// Activating `Jandor's Saddlebags` pays three mana to target and untap the tapped creature upon resolution.
#[test]
fn jandor_s_saddlebags_untaps_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                jandor_s_saddlebags(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bags = on_battlefield(&engine, p0, jandor_s_saddlebags()).expect("bags on battlefield");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("elves on battlefield");

    tap_all_mana_but(&mut engine, p0, Some(jandor_s_saddlebags()));
    assert!(is_tapped(&engine, elves), "elves tapped for mana");
    assert!(!is_tapped(&engine, bags), "saddlebags still untapped");

    activate(&mut engine, p0, jandor_s_saddlebags(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&elves), "creature is a legal target");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("targeted elves");

    pass_until(&mut engine, stack_is_empty);

    assert!(!is_tapped(&engine, elves), "target creature was untapped");
    assert!(is_tapped(&engine, bags), "`Jandor's Saddlebags` is tapped");
}
