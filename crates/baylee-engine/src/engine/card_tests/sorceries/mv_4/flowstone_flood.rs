//! `cards/sorceries/mv_4/flowstone_flood.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn flowstone_flood() -> CardIndex {
    card_index("7f77de5d-1088-42d0-8f68-5633aeab0d74")
}

/// p0 holds Flowstone Flood and `others` over four Mountains at `life`; p1 has
/// a Forest, two Islands and a Counterspell. p0's main phase.
fn a_flood_over_a_forest(others: &[CardIndex], life: i32) -> Engine<RegistryLookup> {
    let mut hand = vec![flowstone_flood()];
    hand.extend_from_slice(others);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .battlefield(1, &[forest(), island(), island()])
        .hand(0, &hand)
        .hand(1, &[counterspell()])
        .life(0, life)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

#[test]
fn flowstone_flood_destroys_target_land() {
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .battlefield(1, &[forest(), gray_ogre()])
        .hand(0, &[flowstone_flood()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, P0));

    let target_land = on_battlefield(&engine, P1, forest()).unwrap();
    cast_from_hand(&mut engine, P0, flowstone_flood());
    let creature = on_battlefield(&engine, P1, gray_ogre()).unwrap();
    kit::settle(&mut engine, P0, |_, p| {
        if let Pending::ChooseTargets { options, .. } = p {
            assert!(!options.contains(&creature));
        }
        kit::choose_targets(p, &[target_land], &[])
    });

    assert!(on_battlefield(&engine, P1, forest()).is_none());
    assert_not_bought_back(&engine, P0, flowstone_flood());
}

/// "Buyback—Pay 3 life, Discard a card at random": both are paid as the spell
/// is cast. With one card beside the spell, chance has one card to choose.
#[test]
fn flowstone_flood_with_its_buyback_paid_comes_back_to_hand() {
    let mut engine = a_flood_over_a_forest(&[gray_ogre()], 20);
    let land = on_battlefield(&engine, P1, forest()).unwrap();

    cast_from_hand(&mut engine, P0, flowstone_flood());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, land);
    assert_eq!(engine.state().players[0].life, 17);
    assert!(in_graveyard(&engine, P0, gray_ogre()).is_some());
    assert!(on_stack(&engine, flowstone_flood()).is_some());
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, forest()).is_some());
    assert_bought_back(&engine, P0, flowstone_flood());
}

/// Declined, nothing extra goes and it is a Flowstone Flood that is spent.
#[test]
fn flowstone_flood_without_its_buyback_goes_to_the_graveyard() {
    let mut engine = a_flood_over_a_forest(&[gray_ogre()], 20);
    let land = on_battlefield(&engine, P1, forest()).unwrap();

    cast_from_hand(&mut engine, P0, flowstone_flood());
    answer_buyback(&mut engine, P0, false);
    aim_at(&mut engine, P0, land);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, forest()).is_some());
    assert_eq!(engine.state().players[0].life, 20);
    assert!(in_hand(&engine, P0, gray_ogre()).is_some());
    assert_not_bought_back(&engine, P0, flowstone_flood());
}

/// Countered, it does not resolve: life and card spent, Flood gone, the land
/// still there.
#[test]
fn flowstone_flood_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = a_flood_over_a_forest(&[gray_ogre()], 20);
    let land = on_battlefield(&engine, P1, forest()).unwrap();

    cast_from_hand(&mut engine, P0, flowstone_flood());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, land);
    counter_the_top_spell(&mut engine, P0, P1);

    assert!(on_battlefield(&engine, P1, forest()).is_some());
    assert_eq!(engine.state().players[0].life, 17);
    assert!(in_graveyard(&engine, P0, gray_ogre()).is_some());
    assert_not_bought_back(&engine, P0, flowstone_flood());
}

/// With its one target gone it does not resolve either, and the cost stays
/// paid.
#[test]
fn flowstone_flood_that_loses_its_target_goes_to_the_graveyard() {
    let mut engine = a_flood_over_a_forest(&[gray_ogre()], 20);
    let land = on_battlefield(&engine, P1, forest()).unwrap();

    cast_from_hand(&mut engine, P0, flowstone_flood());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, land);
    bury(&mut engine, &[land]);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 17);
    assert!(in_graveyard(&engine, P0, gray_ogre()).is_some());
    assert_not_bought_back(&engine, P0, flowstone_flood());
}

/// Both halves must be payable: no card beside the spell, or under 3 life,
/// and nothing is asked.
#[test]
fn flowstone_flood_short_of_a_card_or_of_life_is_not_asked() {
    for mut engine in [
        a_flood_over_a_forest(&[], 20),
        a_flood_over_a_forest(&[gray_ogre()], 2),
    ] {
        cast_from_hand(&mut engine, P0, flowstone_flood());
        assert!(
            matches!(engine.pending(), Pending::ChooseTargets { .. }),
            "got {:?}",
            engine.pending()
        );
    }
}
