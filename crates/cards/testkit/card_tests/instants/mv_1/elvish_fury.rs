//! `cards/instants/mv_1/elvish_fury.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn elvish_fury() -> CardIndex {
    card_index("557d2af0-04c5-461d-826b-7208c4535dc2")
}

/// p0 has `lands` Forests and Grizzly Bears, with Elvish Fury in hand; p1 has
/// Grizzly Bears, two Islands and a Counterspell. p0's main phase.
fn a_fury_on_the_bears(lands: usize) -> Engine<RegistryLookup> {
    let mut board = vec![forest(); lands];
    board.push(index::GRIZZLY_BEARS);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .battlefield(1, &[index::GRIZZLY_BEARS, island(), island()])
        .hand(0, &[elvish_fury()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Elvish Fury (`557d2af0-04c5-461d-826b-7208c4535dc2`) — {G} Instant:
/// "Target creature gets +2/+2 until end of turn."
#[test]
fn elvish_fury_gives_target_creature_plus_two_plus_two_until_end_of_turn() {
    let mut engine = a_fury_on_the_bears(1);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    assert_eq!(pt(&engine, bears), (2, 2));

    cast_from_hand(&mut engine, P0, elvish_fury());
    let land = on_battlefield(&engine, P0, forest()).unwrap();
    let opposing = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    let menu = aim_at(&mut engine, P0, bears);
    assert!(menu.contains(&opposing));
    assert!(!menu.contains(&land));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (4, 4));
    assert_eq!(pt(&engine, opposing), (2, 2));
    assert_not_bought_back(&engine, P0, elvish_fury());

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, bears), (2, 2));
}

/// "Buyback {4}": the {4} is paid on top, and the card comes home.
#[test]
fn elvish_fury_with_its_buyback_paid_comes_back_to_hand() {
    let mut engine = a_fury_on_the_bears(5);
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, elvish_fury());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bears);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{G}} and {{4}} are the five floating"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (4, 4));
    assert_bought_back(&engine, P0, elvish_fury());
}

/// Declined, it costs four less and is an Elvish Fury that is spent.
#[test]
fn elvish_fury_without_its_buyback_goes_to_the_graveyard() {
    let mut engine = a_fury_on_the_bears(5);
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, elvish_fury());
    answer_buyback(&mut engine, P0, false);
    aim_at(&mut engine, P0, bears);
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (4, 4));
    assert_not_bought_back(&engine, P0, elvish_fury());
}

/// Countered, it does not resolve: buyback paid, card gone, the creature as
/// it was.
#[test]
fn elvish_fury_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = a_fury_on_the_bears(5);
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, elvish_fury());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bears);
    counter_the_top_spell(&mut engine, P0, P1);

    assert_eq!(pt(&engine, bears), (2, 2));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_not_bought_back(&engine, P0, elvish_fury());
}

/// With its one target gone it does not resolve either.
#[test]
fn elvish_fury_that_loses_its_target_goes_to_the_graveyard() {
    let mut engine = a_fury_on_the_bears(5);
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, elvish_fury());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bears);
    bury(&mut engine, &[bears]);
    pass_until(&mut engine, stack_is_empty);

    assert_not_bought_back(&engine, P0, elvish_fury());
}
