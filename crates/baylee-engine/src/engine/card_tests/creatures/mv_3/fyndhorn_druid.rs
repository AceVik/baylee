//! `cards/creatures/mv_3/fyndhorn_druid.rs`, played.
//!
//! The dies trigger ("if it was blocked this turn, you gain 4 life") is off the card: nothing reads
//! that this creature was blocked this turn, and gaining the life on every death would be more
//! than printed. This test pins that, so the day the piece exists it has to move (with tests that
//! the life is gained after a block and not after an unblocked attack, a block it made itself, or
//! a death in a quiet turn).

#[allow(clippy::wildcard_imports)]
use super::*;

fn fyndhorn_druid() -> CardIndex {
    card_index("98e51f1a-ed6e-4286-a41b-e4656d08c934")
}

#[test]
fn fyndhorn_druid_is_a_two_two_that_gains_no_life_when_it_dies_unblocked() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), fyndhorn_druid()])
        .hand(0, &[lightning_bolt()])
        .start();

    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let druid = on_battlefield(&engine, p0, fyndhorn_druid()).unwrap();
    assert_eq!(pt(&engine, druid), (2, 2));
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![druid],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, fyndhorn_druid()).is_some(),
        "it died"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "it was not blocked this turn, so no life"
    );
}
