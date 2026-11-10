//! `cards/creatures/mv_2/coral_fighters.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn coral_fighters() -> CardIndex {
    card_index("8a397a64-542f-43fb-bf54-27ac651e1819")
}

/// Coral Fighters (8a397a64-542f-43fb-bf54-27ac651e1819) — {1}{U} 1/1:
/// "Whenever this creature attacks and isn't blocked, look at the top card of
/// defending player's library."
#[test]
fn coral_fighters_looks_at_top_of_defending_players_library_when_unblocked() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[coral_fighters()])
        .start();
    keep_mulligans(&mut engine);

    let fighters = on_battlefield(&engine, P0, coral_fighters()).unwrap();
    let top_before = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(P1))
        .last()
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(fighters, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::LookAtLibrary,
                ..
            }
        )
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options, vec![top_before]);
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(engine.state().players[1].life, 19);
}

/// "attacks and isn't blocked": a blocked Coral Fighters looks at nothing.
/// Any look question would stop `pass_until` ("unexpected while passing").
#[test]
fn coral_fighters_does_not_look_when_it_is_blocked() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[coral_fighters()])
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);

    let fighters = on_battlefield(&engine, P0, coral_fighters()).unwrap();
    let giant = on_battlefield(&engine, P1, HILL_GIANT).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(fighters, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, fighters)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(on_battlefield(&engine, P0, coral_fighters()).is_none());
    assert_eq!(engine.state().players[1].life, 20);
}
