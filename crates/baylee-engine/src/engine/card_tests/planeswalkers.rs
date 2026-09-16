//! Planeswalkers, the door `cards/planeswalkers/` puts them behind:
//! loyalty abilities, and what a walker's own ability may reach.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// Karn, the Great Creator −2: "reveal an artifact card you own from outside
/// the game ... put that card into your hand."
///
/// Also the regression test for the sideboard itself: those cards must be
/// reachable by the wish and absent from the library, which is where they
/// used to end up.
#[test]
fn karn_minus_two_pulls_an_artifact_from_outside_the_game() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[karn_the_great_creator()])
        .sideboard(0, &[chromatic_lantern()])
        .start();
    keep_mulligans(&mut engine);

    let library = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0));
    assert!(
        !library.iter().any(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == chromatic_lantern()))
        }),
        "the sideboard was shuffled into the library"
    );

    reach_main_phase(&mut engine, p0);
    let karn = on_battlefield(&engine, p0, karn_the_great_creator()).expect("karn deployed");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: karn,
                ability_index: 2,
            },
        )
        .unwrap();

    // The ability goes on the stack; the wish is offered when it resolves.
    let mut offered = None;
    for _ in 0..40 {
        match engine.pending().clone() {
            Pending::ChooseCards {
                options, min, max, ..
            } => {
                assert_eq!((min, max), (0, 1), "the wish is optional and singular");
                offered = Some(options);
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while resolving the wish: {other:?}"),
        }
    }
    let offered = offered.expect("the wish offered the sideboard");
    assert_eq!(
        offered.len(),
        1,
        "only the artifact outside the game qualifies"
    );

    let wanted = offered[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wanted],
            },
        )
        .unwrap();
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .contains(&wanted),
        "the wished-for card is in hand"
    );
}

/// Karn, the Great Creator +1: "up to **one target** noncreature artifact
/// becomes an artifact creature with power and toughness each equal to its
/// mana value."
///
/// It animated *every* noncreature artifact on every battlefield, because
/// both halves of the sentence were written with the same filter the
/// targeting used — which reads like the same claim and is not. Pointed at a
/// nought-cost artifact it was a one-sided board wipe: everything it touched
/// became a 0/0 and the next state-based check swept it up. Reported from a
/// game as "all lands and artifacts were removed from all fields, except
/// creatures", which is this filter exactly — an artifact *creature* is not
/// a noncreature artifact and was the only thing left standing.
#[test]
fn karn_plus_one_animates_the_target_and_nothing_else() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(43, forest())
        .battlefield(
            0,
            &[karn_the_great_creator(), mox_opal(), chromatic_lantern()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let karn = on_battlefield(&engine, p0, karn_the_great_creator()).expect("karn deployed");
    let mox = on_battlefield(&engine, p0, mox_opal()).expect("the mox is out");
    let lantern = on_battlefield(&engine, p0, chromatic_lantern()).expect("the lantern is out");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: karn,
                ability_index: 1,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&mox) && options.contains(&lantern),
        "both noncreature artifacts are targetable: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![mox] })
        .unwrap();
    pass_until(&mut engine, |e| on_battlefield(e, p0, mox_opal()).is_none());

    // The mox is a nought-cost artifact, so it animated into a 0/0 and died
    // to a state-based action. That is the rules answer for the card the
    // ability was pointed at, and it is how the fault was noticed at all.
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Graveyard(p0))
            .iter()
            .any(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == mox_opal()))
            }),
        "the target became a 0/0 and was put into its owner's graveyard"
    );
    // And the bystander is untouched: still on the battlefield, and still
    // not a creature. Before the fix it was a 0/0 in the graveyard beside
    // the mox, along with every other noncreature artifact in the game.
    let still = engine
        .state()
        .object(lantern)
        .expect("the lantern was never targeted and is still on the table");
    assert!(
        !still.characteristics().types.intersects(TypeSet::CREATURE),
        "an untargeted artifact was animated too"
    );
    assert!(
        on_battlefield(&engine, p0, chromatic_lantern()).is_some(),
        "and it is still on the battlefield"
    );
}

/// And the −2 offers the sideboard and exile, never the library.
///
/// Filed beside the wish's own test because the two failures look identical
/// from the client: a dialog full of artifact cards the player did not
/// expect. This one fills the library with the very artifact the wish
/// matches, so a version that read `Library(you)` would offer sixty of them.
#[test]
fn karn_minus_two_never_offers_the_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(44, chromatic_lantern())
        .battlefield(0, &[karn_the_great_creator()])
        .sideboard(0, &[mox_opal()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let karn = on_battlefield(&engine, p0, karn_the_great_creator()).expect("karn deployed");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: karn,
                ability_index: 2,
            },
        )
        .unwrap();
    let offered = loop {
        match engine.pending().clone() {
            Pending::ChooseCards { options, .. } => break options,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while resolving the wish: {other:?}"),
        }
    };
    for id in &offered {
        let zone = engine.state().object(*id).map(|o| o.zone);
        assert!(
            matches!(
                zone,
                Some(crate::zone::Zone::OutsideGame | crate::zone::Zone::Exile)
            ),
            "the wish offered a card in {zone:?}"
        );
    }
    assert_eq!(offered.len(), 1, "only the mox is outside the game");
}

/// Aminatou's −1 exiles a permanent you own and returns it — the flicker
/// the owner reported. A tapped land came back tapped.
///
/// CR 400.7: what comes back is a new object with no memory of the old
/// one, and nothing cleared the old one's status, so the tapped bit rode
/// through the exile zone and back. The fix is in
/// [`crate::state::GameState::move_object`], which is the door every zone
/// change goes through, so a bounced creature and a reanimated one are
/// covered by the same three lines.
#[test]
fn a_blinked_permanent_comes_back_untapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[aminatou(), forest(), plains(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Tap the land the only way a player can: use it.
    tap_all_mana(&mut engine, p0);
    let land = on_battlefield(&engine, p0, forest()).expect("the forest is on the battlefield");
    assert!(
        engine
            .state()
            .object(land)
            .expect("the land is there")
            .status
            .contains(crate::object::Status::TAPPED),
        "the land is tapped before the flicker",
    );

    activate(&mut engine, p0, aminatou(), 1);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert!(options.contains(&land), "the tapped land is a legal target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![land],
                players: vec![],
            },
        )
        .expect("the land is targeted");

    pass_until(&mut engine, |e| {
        e.state()
            .object(land)
            .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
            && e.state()
                .zones
                .list(crate::zone::ZoneLocation::Stack)
                .is_empty()
    });
    assert!(
        !engine
            .state()
            .object(land)
            .expect("the land came back")
            .status
            .contains(crate::object::Status::TAPPED),
        "a permanent that changed zones is a new object and enters untapped",
    );
}

/// Jace, the Mind Sculptor's +2: "Look at the top card of **target
/// player's** library. **You** may put that card on the bottom of **that
/// player's** library."
///
/// Two players, two roles, and the engine had each of them on the wrong
/// seat. `Effect::ScryFor` handed the `Pending::ChooseCards` to the target,
/// so the *opponent* decided whether to keep their own card — the opposite
/// of what the card says. And `AwaitingOp::Scry` then bottomed the chosen
/// card into `res.controller`'s library, which is not a misplacement so
/// much as a theft: the card came out of one player's library and went into
/// another's.
///
/// Jace is the only card in the pool that uses `ScryFor`, and nothing in
/// the suite had ever activated it, which is how both survived. The two
/// library counts below are what catch the second one; the asked seat
/// catches the first.
#[test]
fn jace_looks_at_a_targets_library_and_the_controller_decides() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(53, forest())
        .battlefield(0, &[jace_the_mind_sculptor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before = (library_size(&engine, p0), library_size(&engine, p1));
    let top_of_theirs = *engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p1))
        .last()
        .expect("the opponent has a library");

    let jace = on_battlefield(&engine, p0, jace_the_mind_sculptor()).expect("jace deployed");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: jace,
                ability_index: 0,
            },
        )
        .expect("the +2 activates");
    let Pending::ChoosePlayer { options, .. } = engine.pending().clone() else {
        panic!("the +2 asks whose library, got {:?}", engine.pending())
    };
    assert!(options.contains(&p1), "the opponent is a legal target");
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();

    // The ability resolves, and the question it raises goes to Jace's
    // controller — never to the player whose library is being looked at.
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
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "\"you may put that card on the bottom\" — you");
    assert_eq!(options, vec![top_of_theirs], "and it is their top card");
    assert_eq!((min, max), (0, 1), "the \"may\" is the zero minimum");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![top_of_theirs],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. }) && stack_is_empty(e)
    });

    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Library(p1))
            .first()
            .copied(),
        Some(top_of_theirs),
        "the card goes to the bottom of the library it came out of"
    );
    assert_eq!(
        (library_size(&engine, p0), library_size(&engine, p1)),
        before,
        "and no card crossed between the two libraries"
    );
}

/// The owner's report: Venser's +2 flickered a Great Divide Guide, and when
/// it came back at the beginning of the end step the Wartime Protestors
/// standing beside it — "whenever **another Ally** you control enters, put a
/// +1/+1 counter on that creature and it gains haste" — said nothing.
///
/// A permanent returning from exile is a permanent *entering the
/// battlefield* (CR 400.7 makes it a new object, and CR 603.6a has the
/// leaves/enters pair), so every watcher on the board is owed its trigger —
/// the one that returned it is not a private arrangement between two
/// objects. Aminatou's immediate flicker is the same claim on the other
/// path and is held by `a_blinked_permanent_comes_back_untapped`; this is
/// the *delayed* one, which runs out of `Engine::process_delayed` rather
/// than out of an effect.
#[test]
fn an_ally_returning_at_the_end_step_still_rallies_the_board() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(53, forest())
        .battlefield(
            0,
            &[
                venser_the_sojourner(),
                wartime_protestors(),
                great_divide_guide(),
                forest(),
                plains(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let guide = on_battlefield(&engine, p0, great_divide_guide()).expect("the guide is out");
    assert_eq!(
        engine
            .state()
            .object(guide)
            .expect("the guide is on the battlefield")
            .counters
            .get(baylee_cards_dsl::CounterKind::P1P1),
        0,
        "nothing has rallied yet"
    );

    activate(&mut engine, p0, venser_the_sojourner(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(options.contains(&guide), "the guide is a legal target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![guide],
                players: vec![],
            },
        )
        .expect("the guide is targeted");

    // Out to exile first: the return is a separate, delayed thing, and a
    // test that only watched the end result could not tell the two apart.
    pass_until(&mut engine, |e| {
        e.state()
            .object(guide)
            .is_some_and(|o| o.zone == crate::zone::Zone::Exile)
    });

    // Then the end step brings it back, and everything the return sets off
    // has to finish before the assertion.
    pass_until(&mut engine, |e| {
        e.state()
            .object(guide)
            .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
            && e.state()
                .zones
                .list(crate::zone::ZoneLocation::Stack)
                .is_empty()
            && matches!(e.pending(), Pending::Priority { .. })
    });

    assert_eq!(
        engine
            .state()
            .object(guide)
            .expect("the guide came back")
            .counters
            .get(baylee_cards_dsl::CounterKind::P1P1),
        1,
        "the Protestors' rally trigger did not see the Ally come back"
    );
    assert!(
        keywords(&engine, guide).contains(baylee_cards_dsl::KeywordSet::HASTE),
        "and the same trigger grants haste until end of turn"
    );
}
