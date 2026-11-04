//! `cards/sorceries/mv_3/brush_with_death.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn brush_with_death() -> CardIndex {
    card_index("d87e580f-eca1-4f91-962b-82d684cba5e7")
}

/// p0 has `lands` Swamps and Brush with Death in hand; p1 has two Islands and
/// a Counterspell. p0's main phase.
fn a_brush_over(lands: usize) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &vec![swamp(); lands])
        .battlefield(1, &[island(), island()])
        .hand(0, &[brush_with_death()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Brush with Death (`d87e580f-eca1-4f91-962b-82d684cba5e7`) — {2}{B} Sorcery:
/// "Target opponent loses 2 life and you gain 2 life."
#[test]
fn brush_with_death_drains_target_opponent_for_two() {
    let mut engine = a_brush_over(3);

    cast_from_hand(&mut engine, P0, brush_with_death());
    assert!(engine.apply(P0, PlayerAction::ChoosePlayer(P0)).is_err());
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_not_bought_back(&engine, P0, brush_with_death());

    assert_eq!(engine.state().players[1].life, 18);
    assert_eq!(engine.state().players[0].life, 22);
}

/// "Buyback {2}{B}{B}": the {2}{B}{B} is paid on top, and the card comes home.
#[test]
fn brush_with_death_with_its_buyback_paid_comes_back_to_hand() {
    let mut engine = a_brush_over(7);

    cast_from_hand(&mut engine, P0, brush_with_death());
    answer_buyback(&mut engine, P0, true);
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{2}}{{B}} and {{2}}{{B}}{{B}} are the seven floating"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 18);
    assert_eq!(engine.state().players[0].life, 22);
    assert_bought_back(&engine, P0, brush_with_death());
}

/// Declined, it costs four less and is a Brush with Death that is spent.
#[test]
fn brush_with_death_without_its_buyback_goes_to_the_graveyard() {
    let mut engine = a_brush_over(7);

    cast_from_hand(&mut engine, P0, brush_with_death());
    answer_buyback(&mut engine, P0, false);
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 18);
    assert_eq!(engine.state().players[0].life, 22);
    assert_not_bought_back(&engine, P0, brush_with_death());
}

/// Countered, it does not resolve: buyback paid, card gone, no life moved. It
/// targets only an opponent, and an opponent does not leave a duel with the
/// game still going, so no target can leave it (CR 608.2b).
#[test]
fn brush_with_death_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = a_brush_over(7);

    cast_from_hand(&mut engine, P0, brush_with_death());
    answer_buyback(&mut engine, P0, true);
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    counter_the_top_spell(&mut engine, P0, P1);

    assert_eq!(engine.state().players[1].life, 20);
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_not_bought_back(&engine, P0, brush_with_death());
}
