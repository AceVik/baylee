//! `cards/instants/mv_3/repulse.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Repulse — {2}{U} instant: "Return target creature to its owner's hand.
/// Draw a card." Each printed half is where the other could hide, so one cast
/// reads both: the bounce lands the *opponent's* Elf in the *opponent's* hand
/// — a spell that tucked it into the caster's hand would pass every count taken
/// on this side of the table — while the draw is read as the very card that was
/// on top of the library, which a library that merely shrank by one could not
/// stand in for. The Sol Ring is the filter's witness: it is an artifact, so
/// "target creature" may not name it, and it is not what moved.
#[test]
fn repulse_bounces_the_named_creature_to_its_owners_hand_and_draws_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                quiet_artifact(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[repulse()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let rock = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring is out");

    // The top of p0's library, named before anything is cast: the list's last
    // entry is the top, and a draw is a move rather than a count.
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, repulse());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert_eq!((min, max), (1, 1), "one creature, and the spell asks once");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&rock),
        "the Sol Ring is an artifact and no creature: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf across the table was one of the options");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted creature left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "\"to its owner's hand\": the Elf goes back to the seat that owns it, \
         not to the seat that cast the spell"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_none(),
        "and the caster's hand is not where it went"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature the spell did not name never moved"
    );
    assert!(
        in_graveyard(&engine, p0, repulse()).is_some(),
        "the instant resolved and is in its owner's graveyard"
    );

    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "and it is the very card that was on top of the library, which a count \
         alone would not say"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card cast and one card drawn: the hand is the size it was"
    );
}
