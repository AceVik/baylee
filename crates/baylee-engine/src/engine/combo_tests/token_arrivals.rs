//! A token arriving is a permanent entering the battlefield (CR 111.1, CR 603.6a), and Nesting Dovehawk is the card in this pool whose only job is to notice — so it watches every door a token comes through: plain creation, the copy a Rite of Replication makes, and the populate its own combat trigger performs. Populate's own rules are here with it: it is mandatory and cannot be answered with the empty list (CR 701.36), it never reaches the stack with no token to copy, and a creation-doubling replacement makes it fire the Dovehawk twice. Both seats keep a Dovehawk on every board, because "whenever a creature token **you control** enters" reads exactly like "one entered somewhere" until a second bird is standing across the table saying otherwise. What that token then carries is `copied_abilities`, and the doubling itself is `doubling`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// A token arriving on the battlefield is a permanent *entering* it
/// (CR 111.1, CR 603.6a), and Nesting Dovehawk is the card in the pool whose
/// only job is to notice.
///
/// Crib Swap hands the Shapeshifter to the exiled creature's controller, so
/// the token and the two watchers are arranged across the table from each
/// other: their Dovehawk sees a token of theirs enter, mine sees nothing —
/// one card read twice, which is what makes the number mean "you control"
/// rather than "one entered somewhere". This is the plain token path,
/// [`crate::resolve::tokens::create_token`], with no copying anywhere in it.
#[test]
fn a_token_arriving_is_a_permanent_entering_the_battlefield() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(94, forest())
        .battlefield(0, &[plains(), plains(), plains(), nesting_dovehawk()])
        .hand(0, &[crib_swap()])
        .battlefield(1, &[llanowar_elves(), forest(), nesting_dovehawk()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert_eq!(
        counters_on_the_dovehawk(&engine, p0),
        0,
        "neither bird has grown before the spell is cast"
    );
    assert_eq!(counters_on_the_dovehawk(&engine, p1), 0);

    aim_at_their_elf(&mut engine, crib_swap(), false);

    assert_eq!(
        tokens_controlled(&engine, p1),
        1,
        "the Shapeshifter is created under the exiled creature's controller"
    );
    assert_eq!(
        counters_on_the_dovehawk(&engine, p1),
        1,
        "and their Dovehawk saw a creature token of theirs enter — once"
    );
    assert_eq!(
        counters_on_the_dovehawk(&engine, p0),
        0,
        "mine watches my own side of the table and nothing entered it"
    );
}

/// The other door tokens come through: a **copy**, which is created by
/// [`crate::resolve::tokens::create_token_copies`] and never touches the
/// factory the test above exercises.
///
/// The seats are the other way round for the same reason they were that way
/// round there. Rite of Replication creates the copy under the caster's
/// control however far away the creature it copies is, so the counter has to
/// land on *my* bird while theirs — the one standing beside the creature
/// being copied — stays a 2/2.
#[test]
fn the_copy_a_rite_makes_enters_the_battlefield_too() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(95, forest())
        .battlefield(
            0,
            &[island(), island(), island(), island(), nesting_dovehawk()],
        )
        .hand(0, &[rite_of_replication()])
        .battlefield(1, &[llanowar_elves(), forest(), nesting_dovehawk()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    aim_at_their_elf(&mut engine, rite_of_replication(), false);
    let copy = the_copy_on(&engine, p0);

    assert!(
        engine
            .state()
            .object(copy)
            .is_some_and(|o| o.controller == p0),
        "the copy is mine"
    );
    assert_eq!(
        counters_on_the_dovehawk(&engine, p0),
        1,
        "and my Dovehawk saw it enter"
    );
    assert_eq!(
        counters_on_the_dovehawk(&engine, p1),
        0,
        "the copy entered under my control, not next to the creature it copies"
    );
}

/// Nesting Dovehawk grows on the token its own populate made.
///
/// Reported from live play: the combat trigger interrupts the turn, the
/// Shapeshifter token is chosen, the copy arrives — and the Dovehawk does
/// not grow, though it grows for a token that arrives any other way. Both
/// steps are asserted here, so a failure says which of the two is wrong.
#[test]
fn the_dovehawk_grows_on_the_token_its_own_populate_made() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(127, forest())
        .battlefield(
            0,
            &[
                nesting_dovehawk(),
                maskwood_nexus(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let hawk = on_battlefield(&engine, p0, nesting_dovehawk()).expect("the Dovehawk");
    assert_eq!(pt(&engine, hawk), (2, 2), "a printed 2/2");

    // A creature token from somewhere else: the Nexus' own {3}, {T}.
    let nexus = on_battlefield(&engine, p0, maskwood_nexus()).expect("the Nexus");
    activate(&mut engine, p0, nexus);
    settle(&mut engine);
    assert_eq!(
        pt(&engine, hawk),
        (3, 3),
        "a creature token entered, so the Dovehawk grew",
    );

    // And now its own populate, at the beginning of combat.
    for _ in 0..40 {
        if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected on the way to combat: {other:?}"),
        }
    }
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the populate trigger asked for no token: {:?}",
            engine.pending()
        )
    };
    let token = *options
        .first()
        .expect("the Shapeshifter is a creature token it controls");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![token],
            },
        )
        .unwrap();
    settle(&mut engine);

    assert_eq!(
        pt(&engine, hawk),
        (4, 4),
        "the populated copy is a creature token entering too",
    );
}

/// The same populate with Elspeth, Storm Slayer on the battlefield.
///
/// The live report's board had her on it, and she is a replacement on token
/// *creation*: populate then makes two copies, so the Dovehawk's "whenever
/// a creature token you control enters" fires twice.
#[test]
fn a_doubled_populate_grows_the_dovehawk_twice() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(131, forest())
        .battlefield(
            0,
            &[
                nesting_dovehawk(),
                maskwood_nexus(),
                elspeth_storm_slayer(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let hawk = on_battlefield(&engine, p0, nesting_dovehawk()).expect("the Dovehawk");

    let nexus = on_battlefield(&engine, p0, maskwood_nexus()).expect("the Nexus");
    activate(&mut engine, p0, nexus);
    settle(&mut engine);
    assert_eq!(
        pt(&engine, hawk),
        (4, 4),
        "the Nexus' one token is doubled, so two tokens entered",
    );

    for _ in 0..40 {
        if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected on the way to combat: {other:?}"),
        }
    }
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the populate trigger asked for no token: {:?}",
            engine.pending()
        )
    };
    let token = *options.first().expect("a creature token it controls");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![token],
            },
        )
        .unwrap();
    settle(&mut engine);

    assert_eq!(
        pt(&engine, hawk),
        (6, 6),
        "and the populated copy is doubled too",
    );
}

/// Populate is a choice and it is mandatory (CR 701.36).
///
/// The card said "up to one target" (`min: 0`), so the Dovehawk's combat
/// trigger could be answered with the empty list and resolve into nothing
/// while a copyable token stood on the battlefield — and with no token at
/// all it went on the stack anyway and resolved into nothing there too,
/// which is the trigger the owner watched fire and do nothing.
#[test]
fn the_dovehawks_populate_cannot_be_declined() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(137, forest())
        .battlefield(
            0,
            &[
                nesting_dovehawk(),
                maskwood_nexus(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let nexus = on_battlefield(&engine, p0, maskwood_nexus()).expect("the Nexus");
    activate(&mut engine, p0, nexus);
    settle(&mut engine);

    for _ in 0..40 {
        if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("unexpected on the way to combat: {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    let Pending::ChooseTargets { min, options, .. } = engine.pending().clone() else {
        panic!("populate asked nothing: {:?}", engine.pending())
    };
    assert_eq!(
        min, 1,
        "a token is there to copy, so populate must be answered"
    );
    assert!(
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
            .is_err(),
        "the empty answer is what let the trigger resolve into nothing",
    );
    let token = *options.first().expect("the Shapeshifter token");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![token],
            },
        )
        .unwrap();
    settle(&mut engine);
    assert_eq!(
        counters_on_the_dovehawk(&engine, p0),
        2,
        "one counter for the Nexus' token and one for the copy populate made",
    );
}

/// The counter-test: with no creature token to copy, the trigger never
/// reaches the stack at all.
#[test]
fn a_populate_with_nothing_to_copy_never_reaches_the_stack() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(139, forest())
        .battlefield(0, &[nesting_dovehawk(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    for _ in 0..40 {
        // Combat arriving with the question never asked is the whole
        // assertion: the trigger was dropped instead of stacked.
        if matches!(engine.pending(), Pending::ChooseAttackers { .. }) {
            return;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("populate asked something: {:?}", engine.pending())
        };
        assert!(
            stack_is_empty(&engine),
            "a populate with nothing to copy was put on the stack in {:?}",
            engine.state().turn.step,
        );
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    panic!("combat never arrived");
}
