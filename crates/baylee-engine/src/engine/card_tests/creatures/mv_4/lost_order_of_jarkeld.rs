//! `cards/creatures/mv_4/lost_order_of_jarkeld.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn lost_order_of_jarkeld() -> CardIndex {
    card_index("75f047dc-4a51-494d-8765-fa7502d1fd06")
}

/// Lost Order of Jarkeld (75f047dc-4a51-494d-8765-fa7502d1fd06) — {2}{W}{W} Creature — Human Knight:
/// "As this creature enters, choose an opponent."
#[test]
fn lost_order_of_jarkeld_chooses_an_opponent_as_it_enters() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[lost_order_of_jarkeld()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, lost_order_of_jarkeld());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    let Pending::ChoosePlayer { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(
        options,
        vec![P1],
        "ChooseOpponent offers only opponents, not the controller P0"
    );
    engine
        .apply(P0, PlayerAction::ChoosePlayer(P1))
        .expect("P1 is a legal opponent choice");
    pass_until(&mut engine, stack_is_empty);

    let order = on_battlefield(&engine, P0, lost_order_of_jarkeld()).expect("Order entered");
    assert_eq!(
        engine.state().object(order).unwrap().chosen_opponent(),
        Some(P1),
        "chosen opponent is recorded on the permanent"
    );
    assert_eq!(pt(&engine, order), (1, 1));
}
