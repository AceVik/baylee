//! `cards/creatures/mv_6/minion_of_the_wastes.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::power_toughness;

const P0: PlayerId = PlayerId::new(0);

fn minion_of_the_wastes() -> CardIndex {
    card_index("2593944c-1bde-4307-b3ca-f7ecbeb027c1")
}

/// Minion of the Wastes (`2593944c-1bde-4307-b3ca-f7ecbeb027c1`) — {3}{B}{B}{B} Creature — Minion (Partial):
/// "As this creature enters, pay any amount of life. Minion of the Wastes's power and toughness are each equal to the life paid as it entered."
#[test]
fn minion_of_the_wastes_sets_power_and_toughness_to_chosen_number() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[minion_of_the_wastes()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, minion_of_the_wastes());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    engine.apply(P0, PlayerAction::ChooseNumber(7)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let id = on_battlefield(&engine, P0, minion_of_the_wastes()).unwrap();
    assert_eq!(power_toughness(&engine, id), (7, 7));
}
