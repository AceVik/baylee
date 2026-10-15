//! `cards/enchantments/mv_5/dire_undercurrents.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn dire_undercurrents() -> CardIndex {
    card_index("7545adab-b6f8-496d-8c25-416c7ec85fa2")
}

/// Casts `creature` (with `lands` under it), names `target` for the
/// trigger and hands the engine back on the "you may" question.
fn trigger_aimed_at(
    creature: CardIndex,
    lands: &[CardIndex],
    target: PlayerId,
) -> Engine<RegistryLookup> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = vec![dire_undercurrents()];
    board.extend_from_slice(lands);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[creature])
        .hand(1, &[forest(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, creature);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    assert!(player_options.contains(&p0) && player_options.contains(&p1));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![target],
            },
        )
        .expect("either player is a legal target");
    run_until(&mut engine, |e| {
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

/// "Whenever a blue creature you control enters, you may have target player
/// draw a card." I name myself and accept.
#[test]
fn dire_undercurrents_blue_creature_draws_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = trigger_aimed_at(
        killer_whale(),
        &[island(), island(), island(), island(), island()],
        p0,
    );
    let before = hand_len(&engine, p0);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(hand_len(&engine, p0), before + 1);
}

/// "You may": declining draws nothing.
#[test]
fn dire_undercurrents_blue_creature_may_decline() {
    let p0 = PlayerId::new(0);
    let mut engine = trigger_aimed_at(
        killer_whale(),
        &[island(), island(), island(), island(), island()],
        p0,
    );
    let before = hand_len(&engine, p0);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(hand_len(&engine, p0), before);
}

/// "Whenever a black creature you control enters, you may have target
/// player discard a card." I name the opponent and accept; they discard one
/// of their two cards.
#[test]
fn dire_undercurrents_black_creature_makes_the_target_discard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = trigger_aimed_at(festering_goblin(), &[swamp()], p1);
    let before = hand_len(&engine, p1);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    run_until(&mut engine, stack_is_empty);
    assert_eq!(hand_len(&engine, p1), before - 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        1
    );
}
