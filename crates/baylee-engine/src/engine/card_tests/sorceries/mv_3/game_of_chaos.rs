//! `cards/sorceries/mv_3/game_of_chaos.rs`, played.
//!
//! Oracle:
//! - "Flip a coin. If you win the flip, you gain 1 life and target opponent loses 1 life, and you decide whether to flip again. If you lose the flip, you lose 1 life and that opponent gains 1 life, and that player decides whether to flip again. Double the life stakes with each flip."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::event::GameEvent;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn game_of_chaos() -> CardIndex {
    card_index("b6756608-a618-4e26-a090-ea8fc992def1")
}

/// Every coin flip the journal holds, in order, as (who flipped, won).
fn flips(engine: &Engine<RegistryLookup>) -> Vec<(PlayerId, bool)> {
    engine
        .state()
        .journal
        .entries()
        .iter()
        .filter_map(|e| match e.event {
            GameEvent::CoinFlipped { player, won } => Some((player, won)),
            _ => None,
        })
        .collect()
}

/// Cast at the opponent, then resolve to the latest flip winner's decision.
#[track_caller]
fn cast_to_decision(seed: u64) -> Engine<RegistryLookup> {
    let mut engine = lk::game(seed, &[], &[], &[game_of_chaos()], &[]);
    lk::float(&mut engine, P0, &[(ManaColor::Red, 3)]);
    lk::announce(&mut engine, P0, game_of_chaos(), 0, 0, &[], &[P1]);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine
}

/// The flip's winner, and only that player, decides after the life changes.
#[track_caller]
fn assert_decision(engine: &Engine<RegistryLookup>, expected_life: [i32; 2]) -> PlayerId {
    let seen = flips(engine);
    let (flipper, won) = *seen.last().expect("the spell flips before asking");
    assert_eq!(flipper, P0, "the caster makes every flip");
    let chooser = if won { P0 } else { P1 };
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::MayDo,
        ..
    } = engine.pending()
    else {
        panic!("the latest winner decides whether to repeat")
    };
    assert_eq!(*player, chooser);
    assert_eq!(
        [lk::life(engine, P0), lk::life(engine, P1)],
        expected_life,
        "stake {} after {} flip(s)",
        1_i32 << (seen.len() - 1),
        seen.len()
    );
    chooser
}

#[track_caller]
fn answer(engine: &mut Engine<RegistryLookup>, player: PlayerId, again: bool) {
    engine
        .apply(player, PlayerAction::YesNo(again))
        .expect("the flip winner answers their own repeat question");
}

#[track_caller]
fn assert_finished(engine: &Engine<RegistryLookup>) {
    assert!(stack_is_empty(engine));
    assert!(in_graveyard(engine, P0, game_of_chaos()).is_some());
}

#[test]
fn caster_repeats_two_wins_at_doubled_stakes_then_stops() {
    let mut finished = None;
    for seed in 0..128 {
        let mut engine = cast_to_decision(seed);
        if !flips(&engine)[0].1 {
            continue;
        }
        assert_decision(&engine, [21, 19]);
        answer(&mut engine, P0, true);

        if !flips(&engine)[1].1 {
            continue;
        }
        assert_decision(&engine, [23, 17]);
        answer(&mut engine, P0, true);

        if !flips(&engine)[2].1 {
            continue;
        }
        assert_decision(&engine, [27, 13]);
        answer(&mut engine, P0, false);
        lk::settle(&mut engine);
        assert_finished(&engine);
        assert_eq!(flips(&engine).len(), 3, "declining flips no fourth coin");
        finished = Some(engine);
        break;
    }
    assert!(finished.is_some(), "a seed played three consecutive wins");
}

#[test]
fn opponent_repeats_after_the_caster_loses_then_the_caster_can_stop() {
    let mut finished = None;
    for seed in 0..128 {
        let mut engine = cast_to_decision(seed);
        if flips(&engine)[0].1 {
            continue;
        }
        assert_decision(&engine, [19, 21]);
        answer(&mut engine, P1, true);

        if !flips(&engine)[1].1 {
            continue;
        }
        assert_decision(&engine, [21, 19]);
        answer(&mut engine, P0, false);
        lk::settle(&mut engine);
        assert_finished(&engine);
        assert_eq!(flips(&engine).len(), 2);
        finished = Some(engine);
        break;
    }
    assert!(finished.is_some(), "a seed played a loss followed by a win");
}

#[test]
fn opponent_can_decline_the_first_repeat() {
    let mut finished = None;
    for seed in 0..64 {
        let mut engine = cast_to_decision(seed);
        if flips(&engine)[0].1 {
            continue;
        }
        assert_decision(&engine, [19, 21]);
        answer(&mut engine, P1, false);
        lk::settle(&mut engine);
        assert_finished(&engine);
        assert_eq!(flips(&engine).len(), 1, "declining flips no second coin");
        finished = Some(engine);
        break;
    }
    assert!(finished.is_some(), "a seed played an initial loss");
}
