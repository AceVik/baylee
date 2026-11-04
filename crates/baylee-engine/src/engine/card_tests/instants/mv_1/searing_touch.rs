//! `cards/instants/mv_1/searing_touch.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::LLANOWAR_ELVES;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn searing_touch() -> CardIndex {
    card_index("1cd9a3ba-a115-4e59-983d-605e4435f3a8")
}

/// p0 has `lands` Mountains and Searing Touch in hand; p1 has Llanowar Elves,
/// two Islands and a Counterspell. p0's main phase.
fn a_touch_over_the_elves(lands: usize) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &vec![mountain(); lands])
        .battlefield(1, &[LLANOWAR_ELVES, island(), island()])
        .hand(0, &[searing_touch()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// `Searing Touch` ({R}, Instant):
/// deals 1 damage to any target.
#[test]
fn searing_touch_deals_one_damage_to_any_target() {
    let mut engine = a_touch_over_the_elves(1);

    let elves = on_battlefield(&engine, P1, LLANOWAR_ELVES).unwrap();
    cast_from_hand(&mut engine, P0, searing_touch());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, LLANOWAR_ELVES).is_none());
    assert!(in_graveyard(&engine, P1, LLANOWAR_ELVES).is_some());
}

/// "Buyback {4}": the {4} is paid on top, and the card comes home.
#[test]
fn searing_touch_with_its_buyback_paid_comes_back_to_hand() {
    let mut engine = a_touch_over_the_elves(5);
    let elves = on_battlefield(&engine, P1, LLANOWAR_ELVES).unwrap();

    cast_from_hand(&mut engine, P0, searing_touch());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, elves);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{R}} and {{4}} are the five floating"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, LLANOWAR_ELVES).is_some());
    assert_bought_back(&engine, P0, searing_touch());
}

/// Declined, it costs four less and is a Searing Touch that is spent.
#[test]
fn searing_touch_without_its_buyback_goes_to_the_graveyard() {
    let mut engine = a_touch_over_the_elves(5);
    let elves = on_battlefield(&engine, P1, LLANOWAR_ELVES).unwrap();

    cast_from_hand(&mut engine, P0, searing_touch());
    answer_buyback(&mut engine, P0, false);
    aim_at(&mut engine, P0, elves);
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, LLANOWAR_ELVES).is_some());
    assert_not_bought_back(&engine, P0, searing_touch());
}

/// Countered, it does not resolve: buyback paid, card gone, the Elves alive.
#[test]
fn searing_touch_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = a_touch_over_the_elves(5);
    let elves = on_battlefield(&engine, P1, LLANOWAR_ELVES).unwrap();

    cast_from_hand(&mut engine, P0, searing_touch());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, elves);
    counter_the_top_spell(&mut engine, P0, P1);

    assert!(on_battlefield(&engine, P1, LLANOWAR_ELVES).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_not_bought_back(&engine, P0, searing_touch());
}

/// With its one target gone it does not resolve either.
#[test]
fn searing_touch_that_loses_its_target_goes_to_the_graveyard() {
    let mut engine = a_touch_over_the_elves(5);
    let elves = on_battlefield(&engine, P1, LLANOWAR_ELVES).unwrap();

    cast_from_hand(&mut engine, P0, searing_touch());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, elves);
    bury(&mut engine, &[elves]);
    pass_until(&mut engine, stack_is_empty);

    assert_not_bought_back(&engine, P0, searing_touch());
}
