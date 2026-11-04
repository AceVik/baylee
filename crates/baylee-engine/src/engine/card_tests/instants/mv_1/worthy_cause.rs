//! `cards/instants/mv_1/worthy_cause.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn worthy_cause() -> CardIndex {
    card_index("b24063f7-157f-47c0-919a-33856d53b902")
}

/// p0 has `lands` Plains, a Hill Giant and Grizzly Bears, with Worthy Cause in
/// hand; p1 has two Islands and a Counterspell. p0's main phase.
fn a_worthy_cause(lands: usize) -> Engine<RegistryLookup> {
    let mut board = vec![plains(); lands];
    board.extend([HILL_GIANT, GRIZZLY_BEARS]);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &board)
        .battlefield(1, &[island(), island()])
        .hand(0, &[worthy_cause()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Names `creature` for the printed additional cost.
#[track_caller]
fn sacrifice(engine: &mut Engine<RegistryLookup>, creature: ObjectId) {
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![creature],
            },
        )
        .unwrap();
}

/// `Worthy Cause` ({W}, Instant):
/// `Buyback {2}`
/// `As an additional cost to cast this spell, sacrifice a creature.`
/// `You gain life equal to the sacrificed creature's toughness.`
#[test]
fn worthy_cause_sacrifices_a_creature_and_gains_life_equal_to_its_toughness() {
    let mut engine = a_worthy_cause(1);

    let giant = on_battlefield(&engine, P0, HILL_GIANT).unwrap();
    cast_from_hand(&mut engine, P0, worthy_cause());
    sacrifice(&mut engine, giant);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, HILL_GIANT).is_none());
    assert_eq!(engine.state().players[0].life, 23);
}

#[test]
fn no_creature_means_the_additional_cost_cannot_be_paid() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut e = lk::game(SEED, &[], &[], &[worthy_cause()], &[]);
    lk::float(&mut e, P0, &[(ManaColor::White, 5)]);
    let card = in_hand(&e, P0, worthy_cause()).unwrap();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority");
    };
    assert!(
        !legal.castable.contains(&card) && !legal.payable.contains(&card),
        "mana alone cannot pay the sacrifice"
    );
}

/// "Buyback {2}" sits beside the printed sacrifice and changes nothing it
/// reads: the life is the toughness of the creature that was sacrificed, the
/// Bears here, and not the bigger one left on the board.
#[test]
fn worthy_cause_with_its_buyback_paid_gains_by_the_sacrificed_creature_and_comes_back() {
    let mut engine = a_worthy_cause(3);
    let bears = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, worthy_cause());
    answer_buyback(&mut engine, P0, true);
    sacrifice(&mut engine, bears);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{W}} and {{2}} are the three floating"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 22);
    assert!(in_graveyard(&engine, P0, GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, P0, HILL_GIANT).is_some());
    assert_bought_back(&engine, P0, worthy_cause());
}

/// Declined, it costs two less, the sacrifice is still mandatory, and it is a
/// Worthy Cause that is spent.
#[test]
fn worthy_cause_without_its_buyback_goes_to_the_graveyard() {
    let mut engine = a_worthy_cause(3);
    let giant = on_battlefield(&engine, P0, HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, P0, worthy_cause());
    answer_buyback(&mut engine, P0, false);
    sacrifice(&mut engine, giant);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 23);
    assert!(in_graveyard(&engine, P0, HILL_GIANT).is_some());
    assert_not_bought_back(&engine, P0, worthy_cause());
}

/// Countered, it does not resolve: both costs spent, the creature gone, no
/// life gained. It targets nothing, so no target can leave it (CR 608.2b).
#[test]
fn worthy_cause_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = a_worthy_cause(3);
    let giant = on_battlefield(&engine, P0, HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, P0, worthy_cause());
    answer_buyback(&mut engine, P0, true);
    sacrifice(&mut engine, giant);
    counter_the_top_spell(&mut engine, P0, P1);

    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(in_graveyard(&engine, P0, HILL_GIANT).is_some());
    assert_not_bought_back(&engine, P0, worthy_cause());
}
