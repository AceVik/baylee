//! `cards/creatures/mv_5/primaris_eliminator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hyperfrag Round shrinks the creatures of the player it named, and of
/// nobody else.
///
/// "Creatures target player controls get -2/-2 until end of turn" was
/// written as a mode with no target at all and a filter matching every
/// creature on the battlefield — so a 3/2 choosing its own second mode
/// killed itself, the board it had just joined and the opponent's together.
/// The mode targets a player now, and `PumpFilter::controlled_by` is what
/// reads the choice back.
///
/// Three assertions because there are three ways to be wrong: the named
/// seat's creature dies, the caster's does not, and the Eliminator itself
/// is still standing.
#[test]
fn primaris_eliminators_hyperfrag_shrinks_only_the_player_it_named() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(43, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                quiet_creature(),
            ],
        )
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[primaris_eliminator()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let mine = on_battlefield(&engine, p0, quiet_creature()).expect("my own Elves");

    cast_from_hand(&mut engine, p0, primaris_eliminator());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        unreachable!("just checked")
    };
    let hyperfrag = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(1)))
        .expect("the Hyperfrag Round is offered");
    engine
        .apply(p0, PlayerAction::ChooseMode(hyperfrag))
        .unwrap();

    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("Hyperfrag asked for no player — got {:?}", engine.pending())
    };
    assert!(
        player_options.contains(&p1),
        "the opponent is a legal choice for \"target player\"",
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, quiet_creature()).is_none(),
        "the named player's 1/1 took -2/-2",
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "my own creature is not one of theirs",
    );
    assert!(
        on_battlefield(&engine, p0, primaris_eliminator()).is_some(),
        "and a 3/2 does not kill itself with its own second mode",
    );
}
