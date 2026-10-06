//! `cards/creatures/artifacts/mv_3/bottle_gnomes.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Bottle Gnomes` prints `Sacrifice this creature: You gain 3 life.` with `Coverage::Implemented`.
/// In this scenario, seat 0 starts at 15 life with `Bottle Gnomes` on the battlefield.
/// Activating the ability pays the sacrifice cost immediately without needing floating mana or targets,
/// moving the creature to the graveyard, and upon resolution increases seat 0's life total to 18.
#[test]
fn bottle_gnomes_sacrifices_itself_to_gain_three_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .life(0, 15)
        .battlefield(0, &[bottle_gnomes()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert_eq!(engine.state().players[0].life, 15);
    let gnomes = on_battlefield(&engine, p0, bottle_gnomes()).expect("gnomes seated");
    assert_eq!(pt(&engine, gnomes), (1, 3));

    activate(&mut engine, p0, bottle_gnomes(), 0);
    assert!(
        on_battlefield(&engine, p0, bottle_gnomes()).is_none(),
        "paying the sacrifice cost moves `Bottle Gnomes` off the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, bottle_gnomes()).is_some(),
        "`Bottle Gnomes` is in the graveyard as the cost was paid"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 18);
}
