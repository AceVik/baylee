//! `cards/instants/mv_2/allay.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn allay() -> CardIndex {
    card_index("664d982a-5cd8-4289-afb8-fa2e083042c9")
}

/// p0 has `lands` Plains and Allay in hand; p1 has a Crusade, two Islands and
/// a Counterspell. p0's main phase.
fn allay_over_a_crusade(lands: usize) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &vec![plains(); lands])
        .battlefield(1, &[index::CRUSADE, island(), island()])
        .hand(0, &[allay()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Allay (`664d982a-5cd8-4289-afb8-fa2e083042c9`) — {1}{W} Instant:
/// "Buyback {3}. Destroy target enchantment."
#[test]
fn allay_destroys_target_enchantment() {
    let mut engine = allay_over_a_crusade(2);

    let ench = on_battlefield(&engine, P1, index::CRUSADE).unwrap();
    cast_from_hand(&mut engine, P0, allay());
    let land = on_battlefield(&engine, P0, plains()).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("targets")
    };
    assert!(!options.contains(&land));
    aim_at(&mut engine, P0, ench);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, index::CRUSADE).is_none());
    assert!(in_graveyard(&engine, P1, index::CRUSADE).is_some());
    assert_not_bought_back(&engine, P0, allay());
}

/// "Buyback {3}": the {3} is paid on top, and the card comes home.
#[test]
fn allay_with_its_buyback_paid_comes_back_to_hand() {
    let mut engine = allay_over_a_crusade(5);
    let ench = on_battlefield(&engine, P1, index::CRUSADE).unwrap();

    cast_from_hand(&mut engine, P0, allay());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, ench);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{W}} and {{3}} are the five floating"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, index::CRUSADE).is_some());
    assert_bought_back(&engine, P0, allay());
}

/// Declined, it costs three less and is an Allay that is spent.
#[test]
fn allay_without_its_buyback_goes_to_the_graveyard() {
    let mut engine = allay_over_a_crusade(5);
    let ench = on_battlefield(&engine, P1, index::CRUSADE).unwrap();

    cast_from_hand(&mut engine, P0, allay());
    answer_buyback(&mut engine, P0, false);
    aim_at(&mut engine, P0, ench);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, index::CRUSADE).is_some());
    assert_not_bought_back(&engine, P0, allay());
}

/// Countered, it does not resolve: buyback paid, card gone, the Crusade still
/// there.
#[test]
fn allay_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = allay_over_a_crusade(5);
    let ench = on_battlefield(&engine, P1, index::CRUSADE).unwrap();

    cast_from_hand(&mut engine, P0, allay());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, ench);
    counter_the_top_spell(&mut engine, P0, P1);

    assert!(on_battlefield(&engine, P1, index::CRUSADE).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_not_bought_back(&engine, P0, allay());
}

/// With its one target gone it does not resolve either.
#[test]
fn allay_that_loses_its_target_goes_to_the_graveyard() {
    let mut engine = allay_over_a_crusade(5);
    let ench = on_battlefield(&engine, P1, index::CRUSADE).unwrap();

    cast_from_hand(&mut engine, P0, allay());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, ench);
    bury(&mut engine, &[ench]);
    pass_until(&mut engine, stack_is_empty);

    assert_not_bought_back(&engine, P0, allay());
}
