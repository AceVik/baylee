//! `cards/lands/deserts/arid_archway.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Arid Archway: "This land enters tapped. When this land enters, return a
/// land you control to its owner's hand." The bounce is **mandatory** and
/// the land it returns is chosen, so the assertion with teeth is that the
/// choice is offered at all and that what it names actually leaves the
/// battlefield — a trigger that resolved against nothing looks identical to
/// a land that simply entered tapped.
///
/// The archway is itself a land its controller controls, so it is on its own
/// offer, and that is the printing rather than an oversight. The Forest is
/// named instead, which is also what makes the departure readable.
///
/// The surveil rider, which needs *another* Desert to have been returned, is
/// played by the tests below this one; here the Forest is bounced and no
/// Desert is involved.
#[test]
fn arid_archway_enters_tapped_and_bounces_a_land_its_controller_names() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[arid_archway()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let archway = play_land(&mut engine, p0, arid_archway());
    assert!(
        entered_tapped(&engine, archway),
        "\"This land enters tapped\" is unconditional"
    );

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
        panic!("the bounce asks which land, got {:?}", engine.pending())
    };
    assert_eq!(
        player, p0,
        "\"a land you control\" — its controller chooses"
    );
    assert_eq!(
        (min, max),
        (1, 1),
        "the sentence is mandatory and names one land"
    );
    let grove = on_battlefield(&engine, p0, forest()).expect("the Forest is on the board");
    assert!(
        options.contains(&grove),
        "the Forest is a land its controller controls"
    );
    assert!(
        options.contains(&archway),
        "and so is the archway, which the printing does not exclude"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![grove],
            },
        )
        .expect("a land the trigger offered");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p0, forest()).is_none(),
        "the Forest it named left the battlefield"
    );
    assert!(
        in_hand(&engine, p0, forest()).is_some(),
        "\"to its owner's hand\" and not to a graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, arid_archway()).is_some(),
        "and the archway stayed, having named something else"
    );

    // A tapped land makes no mana, so the second sentence waits for the
    // untap step and the main phase after it.
    cross_into_the_next_own_main(&mut engine, p0);
    assert!(
        !is_tapped(&engine, archway),
        "an ordinary untap step untaps it"
    );
    // Ability 1: the ETB trigger is this card's ability 0.
    activate(&mut engine, p0, arid_archway(), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        2,
        "{{T}}: Add {{C}}{{C}} — two, which is what pays for the bounce"
    );
}

/// What the trigger left behind: whether a surveil was asked (and, if so,
/// how many cards it looked at), found by letting the bounce resolve.
struct Settled {
    /// The cards the surveil looked at, when one was asked.
    surveilled: Option<Vec<ObjectId>>,
    /// The size of the controller's library just before the land was played.
    library_before: usize,
}

/// Plays Arid Archway over `board`, bounces `named` and lets the trigger
/// finish, sending the card a surveil looks at to the graveyard.
///
/// Returns the Archway, the object that was named, and what settled.
#[track_caller]
fn archway_bouncing(
    board: &[CardIndex],
    named: CardIndex,
) -> (Engine<RegistryLookup>, ObjectId, ObjectId, Settled) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, board)
        .hand(0, &[arid_archway()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let library_before = library_size(&engine, p0);

    let archway = play_land(&mut engine, p0, arid_archway());
    let target = if named == arid_archway() {
        archway
    } else {
        on_battlefield(&engine, p0, named).expect("the named land is on the board")
    };
    // With the Archway as the only land there is only one thing to return, and
    // a game may or may not ask: either way the land it returns is the Archway.
    // A surveil, if one comes, is answered by sending the looked-at card away.
    let mut surveilled = None;
    for _ in 0..6 {
        pass_until(&mut engine, |e| {
            matches!(
                e.pending(),
                Pending::ChooseCards { .. } | Pending::Arrange { .. }
            ) || at_rest(e, p0)
        });
        match engine.pending().clone() {
            Pending::ChooseCards { options, .. } => {
                assert!(options.contains(&target), "the land named is on the menu");
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![target],
                        },
                    )
                    .expect("a land the trigger offered");
            }
            Pending::Arrange {
                player,
                cards,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "the Archway's controller surveils");
                assert_eq!(
                    prompt,
                    crate::choice::ArrangePrompt::Surveil,
                    "the question is a surveil"
                );
                engine
                    .apply(p0, look_answer(&cards, &cards))
                    .expect("putting the looked-at card into the graveyard is legal");
                surveilled = Some(cards);
            }
            _ => break,
        }
    }
    (
        engine,
        archway,
        target,
        Settled {
            surveilled,
            library_before,
        },
    )
}

/// "If another Desert was returned this way, surveil 1." Returning a Desert
/// (the plain `Desert` card) is a surveil of exactly one card, and the player
/// who sends it away finds it in the graveyard; the Desert is in hand.
#[test]
fn arid_archway_surveils_one_when_it_returns_another_desert() {
    let p0 = PlayerId::new(0);
    let (engine, archway, desert_object, settled) = archway_bouncing(&[desert()], desert());

    let looked_at = settled
        .surveilled
        .clone()
        .expect("a Desert returned: surveil 1");
    assert_eq!(looked_at.len(), 1, "surveil 1 looks at one card");
    assert!(
        in_hand(&engine, p0, desert()).is_some(),
        "the Desert went back to its owner's hand"
    );
    assert!(
        on_battlefield(&engine, p0, desert()).is_none(),
        "and left the battlefield"
    );
    assert!(
        on_battlefield(&engine, p0, arid_archway()).is_some(),
        "the Archway stays"
    );
    assert_ne!(archway, desert_object);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1,
        "the surveilled card went to the graveyard"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .contains(&looked_at[0]),
        "and it is the one that was looked at"
    );
    assert_eq!(
        library_size(&engine, p0),
        settled.library_before - 1,
        "the library lost exactly that one card"
    );
}

/// "If **another Desert** was returned": a land that is not a Desert gives no
/// surveil. The Forest is returned and nothing is looked at, nothing is milled.
#[test]
fn arid_archway_does_not_surveil_when_it_returns_a_land_that_is_not_a_desert() {
    let p0 = PlayerId::new(0);
    let (engine, _archway, _forest, settled) = archway_bouncing(&[forest()], forest());

    assert!(
        settled.surveilled.is_none(),
        "a Forest is not a Desert: no surveil"
    );
    assert!(
        in_hand(&engine, p0, forest()).is_some(),
        "the Forest is back in hand"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .is_empty(),
        "nothing was milled"
    );
    assert_eq!(library_size(&engine, p0), settled.library_before);
}

/// "If **another** Desert was returned": the Archway is itself a Desert, but
/// returning itself (ruling, 2024-04-12: it does so when it is the only land)
/// is not another Desert, so there is no surveil.
#[test]
fn arid_archway_returning_itself_is_not_another_desert() {
    let p0 = PlayerId::new(0);
    let (engine, _archway, _, settled) = archway_bouncing(&[], arid_archway());

    assert!(
        settled.surveilled.is_none(),
        "the Archway is not \"another\" Desert than itself: no surveil"
    );
    assert!(
        in_hand(&engine, p0, arid_archway()).is_some(),
        "the Archway is back in hand"
    );
    assert!(
        on_battlefield(&engine, p0, arid_archway()).is_none(),
        "and no longer on the battlefield"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .is_empty(),
        "nothing was milled"
    );
    assert_eq!(library_size(&engine, p0), settled.library_before);
}
