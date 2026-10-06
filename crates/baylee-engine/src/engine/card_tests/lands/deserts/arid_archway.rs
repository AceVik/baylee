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
/// The file is `Coverage::Partial` for the surveil rider, which needs a
/// second Desert to have been returned — there is one Desert here and the
/// clause could not fire even on a finished card.
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
