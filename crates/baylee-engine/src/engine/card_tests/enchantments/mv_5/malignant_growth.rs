//! `cards/enchantments/mv_5/malignant_growth.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::counters;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "b3235d21-ce52-4a99-a2fe-99b0fe7cc8bc"
fn malignant_growth() -> CardIndex {
    card_index("b3235d21-ce52-4a99-a2fe-99b0fe7cc8bc")
}

#[test]
fn malignant_growth_adds_growth_counter_at_upkeep_and_draws_and_damages_opponent_in_draw_step() {
    let mut engine = lk::game(SEED, &[island()], &[], &[malignant_growth()], &[]);
    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Colorless, 3),
            (ManaColor::Green, 1),
            (ManaColor::Blue, 1),
        ],
    );
    cast_with_floating(&mut engine, P0, malignant_growth());
    pass_until(&mut engine, stack_is_empty);
    let growth = lk::obj(&engine, P0, malignant_growth());

    // Advance to P1's turn, then back to P0's upkeep so Malignant Growth gets a growth counter.
    reach_their_main_phase(&mut engine, P1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    tap_all_mana(&mut engine, P0);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P0),
    );
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    reach_their_main_phase(&mut engine, P0);

    assert_eq!(
        engine
            .state()
            .object(growth)
            .unwrap()
            .counters
            .get(counters::GROWTH),
        1
    );

    let p1_hand_before = engine.state().zones.list(ZoneLocation::Hand(P1)).len();
    reach_their_main_phase(&mut engine, P1);
    let p1_hand_after = engine.state().zones.list(ZoneLocation::Hand(P1)).len();

    assert_eq!(
        p1_hand_after,
        p1_hand_before + 2,
        "P1 drew their normal draw-step card plus 1 extra card from Malignant Growth"
    );
    assert_eq!(
        engine.state().players[1].life,
        19,
        "Malignant Growth dealt 1 damage to P1 for the 1 growth counter"
    );
}

/// "Cumulative upkeep {1}": declined at the first upkeep, Malignant Growth is
/// sacrificed before it has a growth counter's worth of draws to give, and the
/// opponent draws only their normal card.
#[test]
fn malignant_growth_sacrificed_at_a_declined_upkeep_gives_the_opponent_nothing() {
    let mut engine = lk::game(SEED, &[], &[], &[malignant_growth()], &[]);
    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Colorless, 3),
            (ManaColor::Green, 1),
            (ManaColor::Blue, 1),
        ],
    );
    cast_with_floating(&mut engine, P0, malignant_growth());
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(
        (engine.state().turn.number, engine.state().turn.active),
        (3, P0),
        "cumulative upkeep asks at its controller's upkeep"
    );
    lk::yes_no(&mut engine, false);
    lk::settle(&mut engine);
    assert!(in_graveyard(&engine, P0, malignant_growth()).is_some());

    let p1_hand_before = engine.state().zones.list(ZoneLocation::Hand(P1)).len();
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P1)).len(),
        p1_hand_before + 1,
        "only the normal draw"
    );
    assert_eq!(engine.state().players[1].life, 20);
}

/// "Each opponent's draw step": the controller's own draw step draws nothing
/// extra and costs no life, and a turn with no growth counter yet does
/// nothing to the opponent either.
#[test]
fn malignant_growth_leaves_its_controller_and_a_counterless_turn_alone() {
    let mut engine = lk::game(SEED, &[island()], &[], &[malignant_growth()], &[]);
    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Colorless, 3),
            (ManaColor::Green, 1),
            (ManaColor::Blue, 1),
        ],
    );
    cast_with_floating(&mut engine, P0, malignant_growth());
    pass_until(&mut engine, stack_is_empty);

    // Turn 2, before any upkeep of P0's has put a growth counter on it.
    let p1_hand_before = engine.state().zones.list(ZoneLocation::Hand(P1)).len();
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P1)).len(),
        p1_hand_before + 1,
        "no growth counter yet: only the normal draw"
    );
    assert_eq!(engine.state().players[1].life, 20);

    // Turn 3: P0 pays the upkeep, gets the counter, and draws one card in
    // their own draw step.
    let p0_hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    tap_all_mana(&mut engine, P0);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P0),
    );
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    reach_their_main_phase(&mut engine, P0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        p0_hand_before + 1,
        "the controller draws only the normal card"
    );
    assert_eq!(engine.state().players[0].life, 20);
}
