//! `cards/creatures/mv_4/stinging_barrier.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Stinging Barrier` is a 0/4 creature costing `{2}{U}{U}` under `Coverage::Implemented`.
/// It prints defender and "{U}, {T}: This creature deals 1 damage to any target."
/// When activated off an Island targeting an opponent's 1/1 creature (such as `Llanowar Elves`),
/// it taps, deals 1 damage to the target, and destroys it.
#[test]
fn stinging_barrier_pings_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[stinging_barrier(), island()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let barrier = on_battlefield(&engine, p0, stinging_barrier())
        .expect("Stinging Barrier is on the battlefield");
    let elf =
        on_battlefield(&engine, p1, llanowar_elves()).expect("opponent controls Llanowar Elves");
    assert_eq!(pt(&engine, barrier), (0, 4));
    assert!(
        keywords(&engine, barrier).contains(KeywordSet::DEFENDER),
        "Stinging Barrier has defender"
    );

    tap_all_mana_but(&mut engine, p0, Some(stinging_barrier()));
    activate(&mut engine, p0, stinging_barrier(), 0);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Stinging Barrier, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&elf),
        "opponent's elf is an offered target: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "target creature was dealt 1 lethal damage and placed in graveyard"
    );
    assert!(
        is_tapped(&engine, barrier),
        "Stinging Barrier tapped to pay its cost"
    );
}
