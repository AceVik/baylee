//! `cards/instants/mv_1/change_of_heart.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn change_of_heart() -> CardIndex {
    card_index("a8c0b41b-916c-4da9-9c39-a058c91a78f4")
}

/// p0 has `lands` Plains, Grizzly Bears and Savannah Lions, with Change of
/// Heart in hand; p1 has two Islands and a Counterspell. p0's main phase.
fn a_change_of_heart(lands: usize) -> Engine<RegistryLookup> {
    let mut board = vec![plains(); lands];
    board.extend([index::GRIZZLY_BEARS, index::SAVANNAH_LIONS]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .battlefield(1, &[island(), island()])
        .hand(0, &[change_of_heart()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Change of Heart (`a8c0b41b-916c-4da9-9c39-a058c91a78f4`) — {W} Instant:
/// "Target creature can't attack this turn."
#[test]
fn change_of_heart_prevents_target_creature_from_attacking() {
    let mut engine = a_change_of_heart(1);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, change_of_heart());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_not_bought_back(&engine, P0, change_of_heart());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers");
    };
    assert!(!attackers.contains(&bears));
    let lions = on_battlefield(&engine, P0, index::SAVANNAH_LIONS).unwrap();
    assert!(attackers.contains(&lions));
    engine
        .apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert!(lk::attackers_offered(&mut engine).contains(&bears));
}

/// "Buyback {3}": the {3} is paid on top, the creature stays home, and the
/// card comes back.
#[test]
fn change_of_heart_with_its_buyback_paid_comes_back_to_hand() {
    let mut engine = a_change_of_heart(4);
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let lions = on_battlefield(&engine, P0, index::SAVANNAH_LIONS).unwrap();

    cast_from_hand(&mut engine, P0, change_of_heart());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bears);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{W}} and {{3}} are the four floating"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_bought_back(&engine, P0, change_of_heart());
    let attackers = lk::attackers_offered(&mut engine);
    assert!(!attackers.contains(&bears));
    assert!(attackers.contains(&lions));
}

/// Declined, it costs three less and is a Change of Heart that is spent.
#[test]
fn change_of_heart_without_its_buyback_goes_to_the_graveyard() {
    let mut engine = a_change_of_heart(4);
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, change_of_heart());
    answer_buyback(&mut engine, P0, false);
    aim_at(&mut engine, P0, bears);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    pass_until(&mut engine, stack_is_empty);

    assert_not_bought_back(&engine, P0, change_of_heart());
    assert!(!lk::attackers_offered(&mut engine).contains(&bears));
}

/// Countered, it does not resolve: buyback paid, card gone, the creature free
/// to attack.
#[test]
fn change_of_heart_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = a_change_of_heart(4);
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, change_of_heart());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bears);
    counter_the_top_spell(&mut engine, P0, P1);

    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_not_bought_back(&engine, P0, change_of_heart());
    assert!(lk::attackers_offered(&mut engine).contains(&bears));
}

/// With its one target gone it does not resolve either.
#[test]
fn change_of_heart_that_loses_its_target_goes_to_the_graveyard() {
    let mut engine = a_change_of_heart(4);
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, change_of_heart());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bears);
    bury(&mut engine, &[bears]);
    pass_until(&mut engine, stack_is_empty);

    assert_not_bought_back(&engine, P0, change_of_heart());
}
