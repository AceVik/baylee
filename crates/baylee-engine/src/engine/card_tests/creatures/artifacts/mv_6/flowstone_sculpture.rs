//! `cards/creatures/artifacts/mv_6/flowstone_sculpture.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn flowstone_sculpture() -> CardIndex {
    card_index("42b01697-8663-4b53-86d5-699063bd0778")
}

/// Flowstone Sculpture (`42b01697-8663-4b53-86d5-699063bd0778`) — {6} Artifact Creature — Shapeshifter (Partial):
/// "{2}, Discard a card: Put a +1/+1 counter on this creature..."
#[test]
fn flowstone_sculpture_discards_a_card_to_put_a_p1p1_counter_on_itself() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[flowstone_sculpture(), forest(), forest()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let sculpture = on_battlefield(&engine, P0, flowstone_sculpture()).unwrap();
    assert_eq!(pt(&engine, sculpture), (4, 4));

    let card_in_hand = in_hand(&engine, P0, forest()).unwrap();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, flowstone_sculpture(), 0);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![card_in_hand],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, sculpture), (5, 5));
    assert_eq!(
        engine
            .state()
            .object(sculpture)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );
}
