//! `cards/sorceries/mv_2/verdant_touch.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::partials_w2c_kit::power_toughness;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn verdant_touch() -> CardIndex {
    card_index("68845200-5346-473d-becd-27315879443d")
}

/// p0 has `lands` Forests and Verdant Touch in hand; p1 has two Islands and a
/// Counterspell. p0's main phase.
fn a_touch_over(lands: usize) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &vec![forest(); lands])
        .battlefield(1, &[island(), island()])
        .hand(0, &[verdant_touch()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Verdant Touch (`68845200-5346-473d-becd-27315879443d`) — {1}{G} Sorcery:
/// "Target land becomes a 2/2 creature that's still a land."
#[test]
fn verdant_touch_makes_target_land_a_two_two_creature() {
    let mut engine = a_touch_over(2);

    let land = on_battlefield(&engine, P0, forest()).unwrap();
    cast_from_hand(&mut engine, P0, verdant_touch());
    aim_at(&mut engine, P0, land);
    pass_until(&mut engine, stack_is_empty);
    assert_not_bought_back(&engine, P0, verdant_touch());

    assert!(types(&engine, land).contains(TypeSet::CREATURE));
    assert!(types(&engine, land).contains(TypeSet::LAND));
    assert_eq!(power_toughness(&engine, land), (2, 2));
    let other = all_on_battlefield(&engine, P0, forest())
        .into_iter()
        .find(|id| *id != land)
        .unwrap();
    assert!(!types(&engine, other).contains(TypeSet::CREATURE));
    reach_their_main_phase(&mut engine, P1);
    assert!(types(&engine, land).contains(TypeSet::CREATURE));
    assert_eq!(pt(&engine, land), (2, 2));
}

/// "Buyback {3}": the {3} is paid on top, and the card comes home.
#[test]
fn verdant_touch_with_its_buyback_paid_comes_back_to_hand() {
    let mut engine = a_touch_over(5);
    let land = on_battlefield(&engine, P0, forest()).unwrap();

    cast_from_hand(&mut engine, P0, verdant_touch());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, land);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{G}} and {{3}} are the five floating"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(types(&engine, land).contains(TypeSet::CREATURE));
    assert_eq!(power_toughness(&engine, land), (2, 2));
    assert_bought_back(&engine, P0, verdant_touch());
}

/// Declined, it costs three less and is a Verdant Touch that is spent.
#[test]
fn verdant_touch_without_its_buyback_goes_to_the_graveyard() {
    let mut engine = a_touch_over(5);
    let land = on_battlefield(&engine, P0, forest()).unwrap();

    cast_from_hand(&mut engine, P0, verdant_touch());
    answer_buyback(&mut engine, P0, false);
    aim_at(&mut engine, P0, land);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    pass_until(&mut engine, stack_is_empty);

    assert!(types(&engine, land).contains(TypeSet::CREATURE));
    assert_not_bought_back(&engine, P0, verdant_touch());
}

/// Countered, it does not resolve: buyback paid, card gone, the land still
/// only a land.
#[test]
fn verdant_touch_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = a_touch_over(5);
    let land = on_battlefield(&engine, P0, forest()).unwrap();

    cast_from_hand(&mut engine, P0, verdant_touch());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, land);
    counter_the_top_spell(&mut engine, P0, P1);

    assert!(!types(&engine, land).contains(TypeSet::CREATURE));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_not_bought_back(&engine, P0, verdant_touch());
}

/// With its one target gone it does not resolve either.
#[test]
fn verdant_touch_that_loses_its_target_goes_to_the_graveyard() {
    let mut engine = a_touch_over(5);
    let land = on_battlefield(&engine, P0, forest()).unwrap();

    cast_from_hand(&mut engine, P0, verdant_touch());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, land);
    bury(&mut engine, &[land]);
    pass_until(&mut engine, stack_is_empty);

    assert_not_bought_back(&engine, P0, verdant_touch());
}
