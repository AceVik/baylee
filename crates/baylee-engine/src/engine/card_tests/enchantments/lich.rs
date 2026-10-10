//! Lich, every sentence played: the life it takes as it enters, the game
//! it keeps you in at 0 life, the draws a gain becomes, the permanents a
//! point of damage costs, and the game it ends when it leaves.
#[allow(clippy::wildcard_imports)] // Shared real-card test vocabulary.
use super::*;
use crate::event::{Cause, LossReason};
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn float(engine: &mut Engine<RegistryLookup>, color: ManaColor, n: u8) {
    engine.dev_state_mut(P0).unwrap().players[0]
        .mana_pool
        .add(color, n.into());
    engine.refresh_offer();
}

fn lose_everything_but_one_swamp(engine: &mut Engine<RegistryLookup>, lich: ObjectId) {
    let lands: Vec<ObjectId> = engine
        .state()
        .battlefield_view()
        .into_iter()
        .filter(|id| {
            *id != lich
                && engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.controller == P0)
        })
        .collect();
    let state = engine.dev_state_mut(P0).unwrap();
    for land in &lands[1..] {
        state
            .move_object(
                *land,
                ZoneLocation::Graveyard(P0),
                crate::zone::ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }
    engine.refresh_offer();
}

/// p0 with six lands and Lich cast and resolved from hand, at its own
/// first main phase.
fn lich_out() -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(5151, swamp())
        .battlefield(
            0,
            &[swamp(), swamp(), swamp(), swamp(), mountain(), plains()],
        )
        .hand(
            0,
            &[
                index::LICH,
                index::LIGHTNING_BOLT,
                index::DISENCHANT,
                index::RAISE_THE_ALARM,
            ],
        )
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    float(&mut engine, ManaColor::Black, 4);
    cast_with_floating(&mut engine, P0, index::LICH);
    pass_until(&mut engine, stack_is_empty);
    let lich = on_battlefield(&engine, P0, index::LICH).expect("Lich resolved");
    (engine, lich)
}

fn bolt_myself(engine: &mut Engine<RegistryLookup>) {
    float(engine, ManaColor::Red, 1);
    cast_with_floating(engine, P0, index::LIGHTNING_BOLT);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0],
            },
        )
        .expect("any target: p0 may aim the Bolt at themselves");
}

fn hand_size(engine: &Engine<RegistryLookup>) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(P0)).len()
}

/// "As this enchantment enters, you lose life equal to your life total" and
/// "You don't lose the game for having 0 or less life": p0 is at 0 and still
/// playing once every state-based action has been checked.
#[test]
fn lich_takes_the_life_total_as_it_enters_and_zero_life_does_not_lose() {
    let (mut engine, _) = lich_out();
    assert_eq!(engine.state().players[0].life, 0);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    assert!(!engine.state().players[0].has_lost(), "Lich keeps p0 in");
}

/// "If you would gain life, draw that many cards instead": no life is
/// gained, no gain is recorded, and three cards are drawn. Every gain comes
/// through `GameState::change_life`, which is why the door is driven here.
#[test]
fn lich_turns_a_life_gain_into_that_many_draws() {
    let (mut engine, _) = lich_out();
    let before = hand_size(&engine);
    let since = engine.state().journal.entries().last().map_or(0, |e| e.seq);
    engine
        .dev_state_mut(P0)
        .unwrap()
        .change_life(P0, 3, Cause::Effect);
    assert_eq!(engine.state().players[0].life, 0, "no life is gained");
    assert_eq!(hand_size(&engine), before + 3, "three cards instead");
    assert!(
        !engine.state().journal.entries().iter().any(
            |e| e.seq > since && matches!(e.event, crate::event::GameEvent::LifeChanged { .. })
        ),
        "a replaced gain is no gain for a trigger to read"
    );
}

/// "Whenever you're dealt damage, sacrifice that many nontoken permanents":
/// three damage, exactly three chosen, all three sacrificed together, and
/// p0 at −3 is still in the game.
#[test]
fn lich_makes_each_point_of_damage_a_sacrifice() {
    let (mut engine, lich) = lich_out();
    bolt_myself(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!((player, min, max), (P0, 3, 3));
    assert_eq!(options.len(), 7, "six lands and Lich itself are nontoken");
    let victims: Vec<ObjectId> = options
        .iter()
        .copied()
        .filter(|id| *id != lich)
        .take(3)
        .collect();
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: victims.clone(),
            },
        )
        .unwrap();
    for victim in victims {
        assert_eq!(engine.state().object(victim).unwrap().zone, Zone::Graveyard);
    }
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, -3);
    assert!(!engine.state().players[0].has_lost());
    assert_eq!(engine.state().object(lich).unwrap().zone, Zone::Battlefield);
}

/// "… sacrifice that many **nontoken** permanents": with two Soldier tokens
/// on the board beside the six lands and Lich, the question lists the seven
/// cards and neither token. Without a token on the board a filter that
/// forgot the word would list the same seven.
#[test]
fn lich_does_not_let_a_token_pay_for_the_damage() {
    let (mut engine, lich) = lich_out();
    float(&mut engine, ManaColor::White, 2);
    cast_with_floating(&mut engine, P0, index::RAISE_THE_ALARM);
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, P0);
    assert_eq!(tokens.len(), 2, "two Soldiers");
    bolt_myself(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options.len(), 7, "six lands and Lich itself");
    assert!(options.contains(&lich));
    assert!(
        tokens.iter().all(|token| !options.contains(token)),
        "a token is not offered"
    );
}

/// "If you can't, you lose the game": two nontoken permanents against three
/// damage — both are sacrificed, nothing is asked, and p0 loses to the
/// effect rather than to their life total.
#[test]
fn lich_loses_the_game_when_there_is_not_enough_to_sacrifice() {
    let (mut engine, lich) = lich_out();
    bolt_myself(&mut engine);
    lose_everything_but_one_swamp(&mut engine, lich);
    pass_until(&mut engine, |e| e.state().players[0].has_lost());
    assert_eq!(engine.state().players[0].loss, Some(LossReason::Effect));
}

/// "When this enchantment is put into a graveyard from the battlefield, you
/// lose the game" — even above 0 life, where the state-based loss would not
/// have taken p0 out.
#[test]
fn lich_put_into_a_graveyard_loses_the_game_even_above_zero_life() {
    let (mut engine, lich) = lich_out();
    engine.dev_state_mut(P0).unwrap().players[0].life = 5;
    float(&mut engine, ManaColor::White, 2);
    cast_with_floating(&mut engine, P0, index::DISENCHANT);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![lich],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().players[0].has_lost());
    assert_eq!(engine.state().players[0].loss, Some(LossReason::Effect));
}
