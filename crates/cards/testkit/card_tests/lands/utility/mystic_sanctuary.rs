//! `cards/lands/utility/mystic_sanctuary.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mystic Sanctuary: "This land enters tapped unless you control three or
/// more other Islands. When this land enters untapped, you may put target
/// instant or sorcery card from your graveyard on top of your library."
///
/// The library is filled with an instant rather than a basic land, which is
/// what gives `seed_graveyard` something the trigger may legally point at.
/// Both sentences are played at once, because the second is gated on the
/// first: the Sanctuary is the fourth Island here, so it arrives untapped
/// and the trigger has to reach the graveyard.
#[test]
fn a_mystic_sanctuary_off_three_other_islands_replays_an_instant() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(962, eerie_interlude())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[mystic_sanctuary()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p0, 1);
    let buried = in_graveyard(&engine, p0, eerie_interlude()).expect("an instant is in the yard");
    let before = library_size(&engine, p0);

    let sanctuary = play_land(&mut engine, p0, mystic_sanctuary());
    assert!(
        !entered_tapped(&engine, sanctuary),
        "three other Islands is three or more, so the Sanctuary enters untapped"
    );
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "an untapped arrival asks the graveyard question: {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&buried),
        "the instant in the graveyard is one of the answers"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![buried],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. }) && stack_is_empty(e)
    });
    assert!(
        in_graveyard(&engine, p0, eerie_interlude()).is_none(),
        "the card left the graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        before + 1,
        "and it is on top of the library"
    );
}

/// The tapped side of the Sanctuary, one Island fewer.
///
/// "You control" is not the engine's default and the filter has to spell it,
/// so the fourth land on the table here is an Island the *opponent* controls:
/// a filter that had merely counted Islands would read three and let this
/// land in untapped.
#[test]
fn a_mystic_sanctuary_counts_only_its_controllers_islands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(963, eerie_interlude())
        .battlefield(0, &[island(), island()])
        .battlefield(1, &[island(), island()])
        .hand(0, &[mystic_sanctuary()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p0, 1);

    let sanctuary = play_land(&mut engine, p0, mystic_sanctuary());
    assert!(
        entered_tapped(&engine, sanctuary),
        "two of your own Islands are not three, whatever is across the table"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "a tapped arrival asks nothing at all: {:?}",
        engine.pending()
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, eerie_interlude()).is_some(),
        "and the instant stays where it was"
    );
}

/// Mystic Sanctuary prints `({T}: Add {U}.)`, `This land enters tapped unless you control three or more other Islands.`, and `When this land enters untapped, you may put target instant or sorcery card from your graveyard on top of your library.`
///
/// Under `Coverage::Implemented`, controlling three other `island()` lands allows Mystic Sanctuary to enter untapped from hand.
/// Its enters-battlefield trigger fires and targets an instant card (`eerie_interlude()`) in the graveyard.
/// Upon resolution, the targeted card is moved from the graveyard to the top of the library.
#[test]
fn mystic_sanctuary_enters_untapped_and_puts_instant_on_top_of_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, eerie_interlude())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[mystic_sanctuary()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    let buried = in_graveyard(&engine, p0, eerie_interlude()).expect("instant in graveyard");
    let before_lib = library_size(&engine, p0);

    let sanctuary = play_land(&mut engine, p0, mystic_sanctuary());
    assert!(
        !entered_tapped(&engine, sanctuary),
        "three other islands satisfy the enters-untapped condition"
    );

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&buried),
        "instant in graveyard is a legal target"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![buried],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, eerie_interlude()).is_none(),
        "instant card left the graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        before_lib + 1,
        "instant card placed on top of library"
    );
}
