//! `cards/creatures/artifacts/mv_6/composite_golem.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Composite Golem` prints `Sacrifice this creature: Add {{W}}{{U}}{{B}}{{R}}{{G}}.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls the 4/4 artifact creature with an empty mana pool.
/// Activating the mana ability resolves immediately without the stack, sacrificing the golem and adding all five colors of mana.
#[test]
fn composite_golem_sacrifices_for_all_five_colors_of_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[composite_golem()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let golem = on_battlefield(&engine, p0, composite_golem()).expect("composite golem seated");
    assert_eq!(pt(&engine, golem), (4, 4));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, composite_golem(), 0);
    assert!(
        stack_is_empty(&engine),
        "mana ability does not use the stack"
    );
    assert!(
        on_battlefield(&engine, p0, composite_golem()).is_none(),
        "`Composite Golem` was sacrificed as cost"
    );
    assert!(
        in_graveyard(&engine, p0, composite_golem()).is_some(),
        "`Composite Golem` is in the graveyard"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1, "added one white");
    assert_eq!(pool.available(ManaColor::Blue), 1, "added one blue");
    assert_eq!(pool.available(ManaColor::Black), 1, "added one black");
    assert_eq!(pool.available(ManaColor::Red), 1, "added one red");
    assert_eq!(pool.available(ManaColor::Green), 1, "added one green");
    assert_eq!(pool.total(), 5, "exactly five mana floating");
}
