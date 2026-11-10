//! `cards/creatures/mv_6/grassland_crusader.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Grassland Crusader` is a 2/4 creature costing `{5}{W}` under `Coverage::Implemented`.
/// It prints "{T}: Target Elf or Soldier creature gets +2/+2 until end of turn."
/// When activated, only Elf or Soldier creatures are legal targets (such as itself and `Llanowar Elves`),
/// while non-Elf non-Soldier creatures (such as `Desert Drake`) are excluded. The chosen target gets +2/+2.
#[test]
fn grassland_crusader_targets_only_elf_or_soldier_and_pumps() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[grassland_crusader(), llanowar_elves()])
        .battlefield(1, &[desert_drake()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let crusader = on_battlefield(&engine, p0, grassland_crusader())
        .expect("Grassland Crusader is on the battlefield");
    let elf = on_battlefield(&engine, p0, llanowar_elves())
        .expect("Llanowar Elves is on the battlefield");
    let drake =
        on_battlefield(&engine, p1, desert_drake()).expect("Desert Drake is on the battlefield");

    assert_eq!(pt(&engine, elf), (1, 1), "base body is 1/1");

    activate(&mut engine, p0, grassland_crusader(), 0);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Grassland Crusader, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&elf),
        "Elf creature is a legal target: {options:?}"
    );
    assert!(
        options.contains(&crusader),
        "Soldier creature is a legal target: {options:?}"
    );
    assert!(
        !options.contains(&drake),
        "Drake creature is not an Elf or Soldier: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elf), (3, 3), "Elf gets +2/+2 until end of turn");
    assert!(
        is_tapped(&engine, crusader),
        "Grassland Crusader tapped to pay its cost"
    );
}
