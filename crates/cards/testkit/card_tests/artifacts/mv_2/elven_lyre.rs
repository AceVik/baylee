//! `cards/artifacts/mv_2/elven_lyre.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Elven Lyre` prints `{{1}}, {{T}}, Sacrifice this artifact: Target creature gets +2/+2 until end of turn.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Elven Lyre`, a `llanowar_elves()`, and a `forest()`.
/// Floating one mana from the forest pays to activate `Elven Lyre`, targeting the elf creature,
/// which sacrifices the lyre and pumps the creature from (1, 1) to (3, 3) until end of turn.
#[test]
fn elven_lyre_sacrifices_to_pump_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), elven_lyre(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("elves on battlefield");
    let lyre = on_battlefield(&engine, p0, elven_lyre()).expect("lyre on battlefield");
    assert_eq!(pt(&engine, elves), (1, 1));

    tap_mana_where(&mut engine, p0, |id| id != lyre && id != elves);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    activate(&mut engine, p0, elven_lyre(), 0);
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
        .expect("targeted creature");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, elven_lyre()).is_some(),
        "`Elven Lyre` was sacrificed"
    );
    assert_eq!(pt(&engine, elves), (3, 3), "target creature received +2/+2");
}
