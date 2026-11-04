//! `cards/sorceries/mv_2/disturbed_burial.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn disturbed_burial() -> CardIndex {
    card_index("cdbaa21d-5896-4b8b-9a08-f0839ccfddfc")
}

/// p0 has `lands` Swamps and Grizzly Bears in the graveyard, with Disturbed
/// Burial in hand; p1 has two Islands and a Counterspell. p0's main phase,
/// the Bears' id handed back.
fn a_burial_for_the_bears(lands: usize) -> (Engine<RegistryLookup>, ObjectId) {
    let mut board = vec![swamp(); lands];
    board.push(index::GRIZZLY_BEARS);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .battlefield(1, &[island(), island()])
        .hand(0, &[disturbed_burial()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let permanent = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    bury(&mut engine, &[permanent]);
    engine.refresh_offer();
    let bears = in_graveyard(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    (engine, bears)
}

/// Disturbed Burial (`cdbaa21d-5896-4b8b-9a08-f0839ccfddfc`) — {1}{B} Sorcery:
/// "Buyback {3}. Return target creature card from your graveyard to your hand."
#[test]
fn disturbed_burial_returns_target_creature_card_from_graveyard_to_hand() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[index::GRIZZLY_BEARS, swamp(), swamp(), forest()])
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[disturbed_burial()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears_perm = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let land = on_battlefield(&engine, P0, forest()).unwrap();
    let opposing = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();
    bury(&mut engine, &[bears_perm, land, opposing]);
    engine.refresh_offer();
    let bears = in_graveyard(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, disturbed_burial());
    let menu = aim_at(&mut engine, P0, bears);
    assert!(!menu.contains(&land));
    assert!(!menu.contains(&opposing));
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_none());
    assert!(in_hand(&engine, P0, index::GRIZZLY_BEARS).is_some());
    assert_not_bought_back(&engine, P0, disturbed_burial());
}

/// "Buyback {3}": the {3} is paid on top, and the card comes home beside the
/// creature it fetched.
#[test]
fn disturbed_burial_with_its_buyback_paid_comes_back_to_hand() {
    let (mut engine, bears) = a_burial_for_the_bears(5);

    cast_from_hand(&mut engine, P0, disturbed_burial());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bears);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{B}} and {{3}} are the five floating"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, P0, index::GRIZZLY_BEARS).is_some());
    assert_bought_back(&engine, P0, disturbed_burial());
}

/// Declined, it costs three less and is a Disturbed Burial that is spent.
#[test]
fn disturbed_burial_without_its_buyback_goes_to_the_graveyard() {
    let (mut engine, bears) = a_burial_for_the_bears(5);

    cast_from_hand(&mut engine, P0, disturbed_burial());
    answer_buyback(&mut engine, P0, false);
    aim_at(&mut engine, P0, bears);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, P0, index::GRIZZLY_BEARS).is_some());
    assert_not_bought_back(&engine, P0, disturbed_burial());
}

/// Countered, it does not resolve: buyback paid, card gone, and the creature
/// stays where it lay.
#[test]
fn disturbed_burial_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let (mut engine, bears) = a_burial_for_the_bears(5);

    cast_from_hand(&mut engine, P0, disturbed_burial());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bears);
    counter_the_top_spell(&mut engine, P0, P1);

    assert!(in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_not_bought_back(&engine, P0, disturbed_burial());
}

/// With its one target exiled from the graveyard it does not resolve either.
#[test]
fn disturbed_burial_that_loses_its_target_goes_to_the_graveyard() {
    let (mut engine, bears) = a_burial_for_the_bears(5);

    cast_from_hand(&mut engine, P0, disturbed_burial());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bears);
    engine
        .dev_state_mut(P0)
        .unwrap()
        .move_object(
            bears,
            ZoneLocation::Exile(P0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, P0, index::GRIZZLY_BEARS).is_none());
    assert_not_bought_back(&engine, P0, disturbed_burial());
}
