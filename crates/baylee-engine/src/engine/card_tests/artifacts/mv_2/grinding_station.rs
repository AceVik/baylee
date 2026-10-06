//! `cards/artifacts/mv_2/grinding_station.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Grinding Station — {2} artifact: "{T}, Sacrifice an artifact: Target
/// player mills three cards" and "Whenever an artifact enters, you may untap
/// Grinding Station."
///
/// The sacrifice names no artifact of its own, so the engine has to ask which
/// one, and that menu is half the proof: both artifacts this seat controls —
/// the Station is an artifact, so it sits on its own menu — and neither the
/// Elves beside them nor the Sol Ring across the table, which CR 701.21a keeps
/// off it. Eating the Sol Ring rather than the Station is what leaves the
/// second sentence something to do: the Station is still tapped from paying
/// its own cost when a second Sol Ring enters, so nothing but the may-untap
/// trigger can stand it back up.
#[allow(clippy::too_many_lines)] // two printed sentences, and the second needs the first to have happened
#[test]
fn grinding_station_mills_three_for_an_artifact_and_untaps_for_one_entering() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), llanowar_elves()],
        )
        .hand(0, &[grinding_station(), quiet_artifact(), quiet_artifact()])
        // An artifact on the other side of the table: "sacrifice an artifact"
        // is not an invitation to eat somebody else's.
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, grinding_station());
    pass_until(&mut engine, stack_is_empty);
    let station = on_battlefield(&engine, p0, grinding_station()).expect("the Station resolved");
    // The artifact the Station is about to eat.
    cast_from_hand(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, stack_is_empty);
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring resolved");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(station, 0)),
        "{{T}} is paid by an untapped Station with an artifact to eat, so its \
         one line is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p1);
    let graveyard_before = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();

    activate(&mut engine, p0, grinding_station(), 0);

    // The two questions one activation asks: who is milled (CR 601.2c) and
    // which artifact is sacrificed (CR 601.2h). Answered in the order they
    // arrive rather than in the order they are expected.
    let mut asked_whom = false;
    let mut menu: Vec<ObjectId> = Vec::new();
    for _ in 0..12 {
        if asked_whom && !menu.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                player_options,
                ..
            } => {
                assert!(
                    player_options.contains(&p1),
                    "\"target player\" reaches across the table: {player_options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![p1],
                        },
                    )
                    .unwrap();
                asked_whom = true;
            }
            Pending::ChoosePlayer { player, options } => {
                assert!(options.contains(&p1), "both seats are legal: {options:?}");
                engine
                    .apply(player, PlayerAction::ChoosePlayer(p1))
                    .unwrap();
                asked_whom = true;
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::CostSacrifice,
                    "a cost and not a search, which is all a client has to tell \
                     the two apart"
                );
                assert_eq!((min, max), (1, 1), "one artifact, no more and no fewer");
                menu = options;
                let fodder = on_battlefield(&engine, p0, quiet_artifact())
                    .expect("the Sol Ring is still standing to be eaten");
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![fodder],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Station's activation resolves: {other:?}"),
        }
    }
    assert!(asked_whom, "milling \"target player\" is a target choice");
    assert_eq!(
        menu.len(),
        2,
        "the two artifacts this seat controls: {menu:?}"
    );
    assert!(
        menu.contains(&station),
        "the Station is an artifact, so it is on its own menu: {menu:?}"
    );
    assert!(
        menu.contains(&ring),
        "and so is the Sol Ring beside it: {menu:?}"
    );
    assert!(
        !menu.contains(&elves),
        "the Elves are a creature: \"an artifact\" is read, not skipped: {menu:?}"
    );
    assert!(
        !menu.contains(&theirs),
        "a seat sacrifices only what it controls, whatever the filter says: {menu:?}"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, station),
        "tapping the Station paid the other half of the cost"
    );
    assert!(
        on_battlefield(&engine, p0, grinding_station()).is_some(),
        "the Station ate the Sol Ring and not itself"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "and the artifact it ate is in its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p1),
        library_before - 3,
        "\"target player mills three cards\""
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        graveyard_before + 3,
        "three cards off the top of that player's library and into their graveyard"
    );

    // The second printed sentence. The Station is tapped and stays that way
    // until an artifact enters; the Sol Ring still in hand is that artifact.
    cast_from_hand(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, |e| !is_tapped(e, station));
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "a second Sol Ring resolved"
    );
    assert!(
        !is_tapped(&engine, station),
        "\"whenever an artifact enters, you may untap this artifact\""
    );
}
