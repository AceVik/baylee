//! `cards/lands/utility/witch_s_cottage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Witch's Cottage is the same pair of sentences over Swamps and creature
/// cards, and it is here because the two lands write the second sentence two
/// different ways -- the Sanctuary as a filter on the trigger's own event,
/// the Cottage as `Condition::SourceMatches(Untapped)`. Whether those two
/// spellings behave alike is exactly what a played test says and a read of
/// either file does not.
#[test]
fn a_witch_s_cottage_off_three_other_swamps_replays_a_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(964, quiet_creature())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[witch_s_cottage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p0, 1);
    let buried = in_graveyard(&engine, p0, quiet_creature()).expect("a creature is in the yard");
    let before = library_size(&engine, p0);

    let cottage = play_land(&mut engine, p0, witch_s_cottage());
    assert!(
        !entered_tapped(&engine, cottage),
        "three other Swamps is three or more, so the Cottage enters untapped"
    );
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "an untapped arrival asks the graveyard question: {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&buried),
        "the creature card in the graveyard is one of the answers"
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
        in_graveyard(&engine, p0, quiet_creature()).is_none(),
        "the card left the graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        before + 1,
        "and it is on top of the library"
    );
}

/// The tapped side of the Cottage, and the half that says the condition is
/// read when the trigger would go on the stack rather than only on
/// resolution: a land that came down tapped asks nothing.
#[test]
fn a_witch_s_cottage_that_enters_tapped_leaves_the_graveyard_alone() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(965, quiet_creature())
        .battlefield(0, &[swamp(), swamp()])
        .battlefield(1, &[swamp(), swamp()])
        .hand(0, &[witch_s_cottage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p0, 1);

    let cottage = play_land(&mut engine, p0, witch_s_cottage());
    assert!(
        entered_tapped(&engine, cottage),
        "two of your own Swamps are not three, whatever is across the table"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "a tapped arrival asks nothing at all: {:?}",
        engine.pending()
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, quiet_creature()).is_some(),
        "and the creature card stays where it was"
    );
}

/// `Witch's Cottage` prints `({{T}}: Add {{B}}.)`, `This land enters tapped unless you control three or more other Swamps.`, and `When this land enters untapped, you may put target creature card from your graveyard on top of your library.`
///
/// Marked `Coverage::Implemented`, playing `Witch's Cottage` while controlling three other `swamp()` lands satisfies `EnterModifier::TappedUnlessCount`, entering untapped.
/// The arrival trigger targets a creature card in the graveyard via `Pending::ChooseTargets`, prompts `Pending::YesNo` for `Effect::MayDo`, and places the creature on top of `ZoneLocation::Library`.
/// Finally, tapping it confirms it produces `{{B}}` as a `subtypes::land::SWAMP`.
#[test]
fn witch_s_cottage_enters_untapped_with_three_swamps_and_recovers_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(964, llanowar_elves())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[witch_s_cottage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p0, 1);
    let buried = in_graveyard(&engine, p0, llanowar_elves()).expect("creature in graveyard");
    let initial_library_size = library_size(&engine, p0);

    let cottage = play_land(&mut engine, p0, witch_s_cottage());
    assert!(
        !entered_tapped(&engine, cottage),
        "controlling three other Swamps enters Witch's Cottage untapped"
    );

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&buried));
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
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. }) && stack_is_empty(e)
    });

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_none(),
        "creature left the graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        initial_library_size + 1,
        "creature returned to the top of the library"
    );
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .expect("library non-empty");
    assert_eq!(top, buried);

    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 4, "three Swamps and Witch's Cottage tap for mana");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        4
    );
}
