//! `cards/creatures/mv_2/dwarven_lieutenant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Dwarven Lieutenant` prints an activated ability `{{1}}{{R}}` targeting a Dwarf creature
/// to grant +1/+0 until end of turn under `Coverage::Implemented`.
/// On a board containing both the Lieutenant (a Dwarf) and an Elf (non-Dwarf),
/// `Pending::ChooseTargets` offers the Lieutenant while strictly excluding the Elf.
/// Upon resolution, the Lieutenant's `pt` becomes (2, 2).
#[test]
fn dwarven_lieutenant_targets_only_dwarf() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1612, forest())
        .battlefield(
            0,
            &[
                dwarven_lieutenant(),
                llanowar_elves(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lieutenant =
        on_battlefield(&engine, p0, dwarven_lieutenant()).expect("lieutenant is seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf is seated");
    assert_eq!(pt(&engine, lieutenant), (1, 2));

    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, dwarven_lieutenant(), 0);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice for Dwarven Lieutenant");
    };
    assert!(options.contains(&lieutenant), "Dwarf is a legal target");
    assert!(
        !options.contains(&elf),
        "non-Dwarf Elf is not a legal target"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![lieutenant],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, lieutenant), (2, 2));
}
