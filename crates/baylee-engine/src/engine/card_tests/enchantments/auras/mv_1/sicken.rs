//! `cards/enchantments/auras/mv_1/sicken.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sicken is `{B}` Aura: "Enchant creature. Enchanted creature gets -1/-1.
/// Cycling {2}". The static is the whole card once it lands, so the scenario
/// has to show both halves of what a `Filter::AttachedToBySource` means — the
/// creature it holds shrinks and nothing else does — and it casts across the
/// table, because "enchant creature" names no controller and a version that
/// quietly required one would pass every test played on its own board. The
/// exact `(5, 5)` is what separates `-1/-1` from a destroy or a `-2/-2`, and
/// the enchanted permanent still being on the battlefield is what makes the
/// body readable at all.
#[test]
fn sicken_shrinks_the_creature_it_enchanters_across_the_table() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), llanowar_elves()])
        .battlefield(1, &[rootbreaker_wurm()])
        .hand(0, &[sicken()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("the Wurm is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    assert_eq!(pt(&engine, wurm), (6, 6), "a printed 6/6 before the Aura");
    assert_eq!(pt(&engine, elves), (1, 1), "and a printed 1/1");

    cast_from_hand(&mut engine, p0, sicken());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "`enchant creature` is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&wurm) && options.contains(&elves),
        "\"enchant creature\" reaches either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .expect("the Wurm was one of the options");
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, sicken()).expect("the Aura resolved onto the table");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(wurm),
        "an Aura enters attached to the creature it was cast on (CR 303.4f)"
    );
    assert_eq!(
        pt(&engine, wurm),
        (5, 5),
        "one less power and one less toughness, so the static is exactly \
         `-1/-1` and not a destroy or a `-2/-2`"
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "the creature it did not enchant keeps its printed numbers, which is \
         `AttachedToBySource` and not \"creatures\""
    );
}
