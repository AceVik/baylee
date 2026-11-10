//! `cards/sorceries/mv_1/mind_peel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mind_peel() -> CardIndex {
    card_index("4f424d94-1544-4086-a57d-2add17332094")
}

/// p0 has `lands` Swamps and Mind Peel in hand; p1 has two Islands, with
/// Grizzly Bears and a Counterspell in hand. p0's main phase.
fn a_peel_at_the_bears(lands: usize) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &vec![swamp(); lands])
        .battlefield(1, &[island(), island()])
        .hand(0, &[mind_peel()])
        .hand(1, &[index::GRIZZLY_BEARS, counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Runs the stack out, p1 discarding the Bears.
#[track_caller]
fn p1_discards_the_bears(engine: &mut Engine<RegistryLookup>) {
    let bears = in_hand(engine, P1, index::GRIZZLY_BEARS).unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
}

/// Mind Peel (`4f424d94-1544-4086-a57d-2add17332094`) — {B} Sorcery:
/// "Target player discards a card."
#[test]
fn mind_peel_causes_target_player_to_discard_a_card() {
    let mut engine = a_peel_at_the_bears(1);

    cast_from_hand(&mut engine, P0, mind_peel());
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    p1_discards_the_bears(&mut engine);
    assert_not_bought_back(&engine, P0, mind_peel());

    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
}

/// "Buyback {2}{B}{B}": the {2}{B}{B} is paid on top, and the card comes home.
#[test]
fn mind_peel_with_its_buyback_paid_comes_back_to_hand() {
    let mut engine = a_peel_at_the_bears(5);

    cast_from_hand(&mut engine, P0, mind_peel());
    answer_buyback(&mut engine, P0, true);
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{B}} and {{2}}{{B}}{{B}} are the five floating"
    );
    p1_discards_the_bears(&mut engine);

    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert_bought_back(&engine, P0, mind_peel());
}

/// Declined, it costs four less and is a Mind Peel that is spent.
#[test]
fn mind_peel_without_its_buyback_goes_to_the_graveyard() {
    let mut engine = a_peel_at_the_bears(5);

    cast_from_hand(&mut engine, P0, mind_peel());
    answer_buyback(&mut engine, P0, false);
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
    p1_discards_the_bears(&mut engine);

    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert_not_bought_back(&engine, P0, mind_peel());
}

/// Countered, it does not resolve: buyback paid, card gone, the Bears still
/// in hand. It targets only a player, and a player does not leave a duel with
/// the game still going, so no target can leave it (CR 608.2b).
#[test]
fn mind_peel_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = a_peel_at_the_bears(5);

    cast_from_hand(&mut engine, P0, mind_peel());
    answer_buyback(&mut engine, P0, true);
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    counter_the_top_spell(&mut engine, P0, P1);

    assert!(in_hand(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_not_bought_back(&engine, P0, mind_peel());
}
