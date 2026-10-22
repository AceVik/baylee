//! `cards/creatures/mv_3/pallimud.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::power_toughness;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn pallimud() -> CardIndex {
    card_index("cad0f865-62e0-47ba-84ae-8e97cb52bbd1")
}

/// Pallimud (`cad0f865-62e0-47ba-84ae-8e97cb52bbd1`) — {2}{R} Creature — Beast (Partial):
/// "As this creature enters, choose an opponent. Pallimud's power is equal to the number of tapped lands the chosen player controls."
#[test]
fn pallimud_sets_power_to_number_of_tapped_opponent_lands() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &[forest(), forest()])
        .hand(0, &[pallimud()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // P0 casts Pallimud; while it is on the stack, P1 taps both Forests.
    cast_from_hand(&mut engine, P0, pallimud());
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, P1);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let id = on_battlefield(&engine, P0, pallimud()).unwrap();
    assert_eq!(power_toughness(&engine, id), (2, 3));
}
