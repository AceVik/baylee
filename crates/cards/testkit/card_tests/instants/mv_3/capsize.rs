//! `cards/instants/mv_3/capsize.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn capsize() -> CardIndex {
    card_index("77637eff-2963-4402-88f3-ca346f762fc8")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// p0 has `lands` Islands and Capsize in hand; p1 has Grizzly Bears, two
/// Islands and a Counterspell. p0's main phase.
fn a_capsize_over_a_bear(lands: usize) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &vec![island(); lands])
        .battlefield(1, &[grizzly_bears(), island(), island()])
        .hand(0, &[capsize()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Capsize (77637eff-2963-4402-88f3-ca346f762fc8) — {1}{U}{U} Instant:
/// "Return target permanent to its owner's hand."
#[test]
fn capsize_returns_target_creature_to_its_owners_hand() {
    let mut engine = a_capsize_over_a_bear(3);
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    cast_from_hand(&mut engine, P0, capsize());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, grizzly_bears()).is_none());
    assert!(in_hand(&engine, P1, grizzly_bears()).is_some());
}

/// "Target permanent": a land is a legal target, and so is one of your own.
#[test]
fn capsize_may_return_any_permanent_including_a_land_of_the_casters() {
    let mut engine = a_capsize_over_a_bear(3);
    // Capsize taps the three Islands for its cost, so aim at a tapped one.
    let island_id = on_battlefield(&engine, P0, island()).unwrap();
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    cast_from_hand(&mut engine, P0, capsize());
    let menu = aim_at(&mut engine, P0, island_id);
    assert!(menu.contains(&island_id) && menu.contains(&bears));
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, P0, island()).is_some());
    assert!(
        on_battlefield(&engine, P1, grizzly_bears()).is_some(),
        "only the target was returned"
    );
}

/// "Buyback {3}": the {3} is paid on top, and the card comes home.
#[test]
fn capsize_with_its_buyback_paid_comes_back_to_hand() {
    let mut engine = a_capsize_over_a_bear(6);
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    cast_from_hand(&mut engine, P0, capsize());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bears);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{U}}{{U}} and {{3}} are the six floating"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, P1, grizzly_bears()).is_some());
    assert_bought_back(&engine, P0, capsize());
}

/// Declined, it costs three less and is a Capsize that is spent.
#[test]
fn capsize_without_its_buyback_goes_to_the_graveyard() {
    let mut engine = a_capsize_over_a_bear(6);
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    cast_from_hand(&mut engine, P0, capsize());
    answer_buyback(&mut engine, P0, false);
    aim_at(&mut engine, P0, bears);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, P1, grizzly_bears()).is_some());
    assert_not_bought_back(&engine, P0, capsize());
}

/// Countered, it does not resolve: buyback paid, card gone.
#[test]
fn capsize_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = a_capsize_over_a_bear(6);
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    cast_from_hand(&mut engine, P0, capsize());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bears);
    counter_the_top_spell(&mut engine, P0, P1);

    assert!(on_battlefield(&engine, P1, grizzly_bears()).is_some());
    assert_not_bought_back(&engine, P0, capsize());
}

/// With its one target gone it does not resolve either.
#[test]
fn capsize_that_loses_its_target_goes_to_the_graveyard() {
    let mut engine = a_capsize_over_a_bear(6);
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    cast_from_hand(&mut engine, P0, capsize());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bears);
    bury(&mut engine, &[bears]);
    pass_until(&mut engine, stack_is_empty);

    assert_not_bought_back(&engine, P0, capsize());
}
