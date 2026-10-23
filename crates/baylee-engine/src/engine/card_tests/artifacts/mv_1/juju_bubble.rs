//! `cards/artifacts/mv_1/juju_bubble.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn juju_bubble() -> CardIndex {
    card_index("9344ae2e-cb2e-400d-9157-e8158ce31813")
}

/// Passes until somebody is asked whether to pay (the cumulative upkeep) and says for how much.
#[track_caller]
fn walk_to_price(engine: &mut Engine<RegistryLookup>) -> baylee_core::mana::ManaCost {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayMana { cost },
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, P0, "the controller is asked");
    cost
}

/// Says yes to the price and pays it with lands.
#[track_caller]
fn pay(engine: &mut Engine<RegistryLookup>) {
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(engine, P0);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(engine, stack_is_empty);
}

/// Casts the Bubble off the board's lands and lets it resolve. It comes in after P0's upkeep, so
/// the first cumulative upkeep question is the one on P0's next turn.
#[track_caller]
fn cast_the_bubble(engine: &mut Engine<RegistryLookup>) -> ObjectId {
    cast_from_hand(engine, P0, juju_bubble());
    pass_until(engine, stack_is_empty);
    on_battlefield(engine, P0, juju_bubble()).expect("the Bubble resolved")
}

/// Juju Bubble ({1} artifact): "{2}: You gain 1 life." and "When you play a card, sacrifice this
/// artifact." A spell cast by the controller is a card played.
#[test]
fn juju_bubble_gains_life_and_sacrifices_when_you_cast_a_spell() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[juju_bubble(), GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, juju_bubble());
    pass_until(&mut engine, stack_is_empty);
    let bubble = on_battlefield(&engine, P0, juju_bubble()).unwrap();

    activate(&mut engine, P0, juju_bubble(), 3);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 21);
    assert!(
        on_battlefield(&engine, P0, juju_bubble()).is_some(),
        "gaining life is not playing a card: the Bubble stays"
    );

    // Casting Grizzly Bears triggers Juju Bubble's sacrifice ability.
    cast_with_floating(&mut engine, P0, GRIZZLY_BEARS);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(bubble).unwrap().zone, Zone::Graveyard);
}

/// "When you play a card": a land is a card played too.
#[test]
fn juju_bubble_goes_to_the_graveyard_when_its_controller_plays_a_land() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[juju_bubble(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let bubble = cast_the_bubble(&mut engine);

    let land = in_hand(&engine, P0, forest()).unwrap();
    engine
        .apply(P0, PlayerAction::PlayLand { card: land })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(bubble).unwrap().zone,
        Zone::Graveyard,
        "playing a land is playing a card"
    );
}

/// Only "you" play cards for the Bubble: the opponent's land, and the opponent's upkeep, leave it.
#[test]
fn juju_bubble_ignores_a_land_the_opponent_plays() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[juju_bubble()])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    cast_the_bubble(&mut engine);

    reach_their_main_phase(&mut engine, P1);
    let their_land = in_hand(&engine, P1, forest()).unwrap();
    engine
        .apply(P1, PlayerAction::PlayLand { card: their_land })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, P0, juju_bubble()).is_some(),
        "the opponent playing a land is not you playing a card"
    );
}

/// "Cumulative upkeep {1}": one age counter and {1} on the first upkeep, two counters and {2}
/// on the second; a refused price sacrifices the Bubble.
#[test]
fn juju_bubble_cumulative_upkeep_grows_by_one_each_turn_and_sacrifices_when_unpaid() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[juju_bubble()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let bubble = cast_the_bubble(&mut engine);
    let age = baylee_cards_dsl::counters::AGE;

    // Through the opponent's whole turn nothing is asked: the first question is P0's own upkeep.
    assert_eq!(
        walk_to_price(&mut engine),
        baylee_core::mana!("{1}"),
        "one counter, {{1}}"
    );
    assert_eq!(engine.state().turn.active, P0, "on P0's own upkeep");
    assert_eq!(counters_on(&engine, bubble, age), 1);
    pay(&mut engine);
    assert!(on_battlefield(&engine, P0, juju_bubble()).is_some());

    assert_eq!(
        walk_to_price(&mut engine),
        baylee_core::mana!("{2}"),
        "two counters, {{2}}"
    );
    assert_eq!(counters_on(&engine, bubble, age), 2);
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(bubble).unwrap().zone,
        Zone::Graveyard,
        "the unpaid upkeep sacrificed it"
    );
}
