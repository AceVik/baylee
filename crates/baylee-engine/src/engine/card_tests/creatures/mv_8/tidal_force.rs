//! `cards/creatures/mv_8/tidal_force.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn tidal_force() -> CardIndex {
    card_index("1b25e262-e2df-4768-b55e-1b7b8d3ee993")
}

/// Tidal Force is a 7/7 for {5}{U}{U}{U}.
#[test]
fn tidal_force_is_cast_for_eight_as_a_seven_seven() {
    let mut engine = at_main(&[island(); 8], &[tidal_force()]);
    let force = cast_and_settle(&mut engine, tidal_force());
    assert_eq!(pt(&engine, force), (7, 7));
}

/// Answers whatever the upkeep trigger asks: names `target`, says `yes` to
/// every "may".
fn drive_upkeep(engine: &mut Engine<RegistryLookup>, target: ObjectId, yes: bool) -> usize {
    let p0 = PlayerId::new(0);
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, p0);
    assert!(options.contains(&target), "any permanent: {options:?}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    let mut questions = 0;
    loop {
        pass_until(engine, |e| {
            matches!(e.pending(), Pending::YesNo { .. })
                || (stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. }))
        });
        let Pending::YesNo { player, .. } = engine.pending().clone() else {
            break;
        };
        questions += 1;
        engine.apply(player, PlayerAction::YesNo(yes)).unwrap();
    }
    questions
}

/// "At the beginning of each upkeep, you may tap or untap target permanent":
/// the Forest of whoever's turn the first upkeep is gets tapped when the
/// player says yes. That land stays tapped through the other player's upkeep
/// (its owner untaps only on their own turn), so naming it again there
/// untaps it: the same trigger both taps and untaps.
#[test]
fn tidal_force_taps_then_untaps_the_named_permanent() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tidal_force(), forest()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    let first = engine.state().turn.active;
    let land = on_battlefield(&engine, first, forest()).expect("the Forest");
    assert!(!is_tapped(&engine, land));
    drive_upkeep(&mut engine, land, true);
    assert!(is_tapped(&engine, land), "tapped");
    drive_upkeep(&mut engine, land, true);
    assert!(!is_tapped(&engine, land), "untapped again");
}

/// Declining the "may" leaves the permanent alone.
#[test]
fn tidal_force_may_decline() {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tidal_force()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    let land = on_battlefield(&engine, p1, forest()).expect("their Forest");
    let questions = drive_upkeep(&mut engine, land, false);
    assert!(questions >= 1, "a may-question was asked");
    assert!(!is_tapped(&engine, land), "declined, so untouched");
}
