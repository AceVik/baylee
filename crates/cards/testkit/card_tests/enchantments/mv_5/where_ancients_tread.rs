//! `cards/enchantments/mv_5/where_ancients_tread.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn where_ancients_tread() -> CardIndex {
    card_index("fca2fcab-4f17-448d-bf6d-f6c913159df8")
}

/// Casts Rootbreaker Wurm (power 6) off seven Forests, points the trigger
/// at the opponent and hands the engine back on its "you may" question.
fn the_wurm_enters() -> Engine<RegistryLookup> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                where_ancients_tread(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[rootbreaker_wurm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, rootbreaker_wurm());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, p0);
    assert!(player_options.contains(&p1), "any target includes players");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent is a legal target");
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

/// "Whenever a creature you control with power 5 or greater enters, you may
/// have this enchantment deal 5 damage to any target." Accepted: 5 damage.
#[test]
fn where_ancients_tread_deals_five_when_a_big_creature_enters() {
    let mut engine = the_wurm_enters();
    engine
        .apply(PlayerId::new(0), PlayerAction::YesNo(true))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 15);
}

/// "You may": declining deals nothing.
#[test]
fn where_ancients_tread_may_decline() {
    let mut engine = the_wurm_enters();
    engine
        .apply(PlayerId::new(0), PlayerAction::YesNo(false))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 20);
}

/// A 2/2 entering does not trigger it: no target is ever asked for.
#[test]
fn where_ancients_tread_ignores_small_creatures() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[where_ancients_tread(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, gray_ogre());
    // `pass_until` panics on a target question it was not told to expect.
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, gray_ogre()).is_some()
    });
    assert_eq!(engine.state().players[1].life, 20);
}
