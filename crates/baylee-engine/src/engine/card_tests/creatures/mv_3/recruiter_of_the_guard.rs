//! `cards/creatures/mv_3/recruiter_of_the_guard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "d521a329-a53a-4962-810a-2abed80df260"

/// Recruiter of the Guard — {2}{W} for a 1/1 Human Soldier whose
/// enters-ability reads "you may search your library for a creature card with
/// toughness 2 or less, reveal it, put it into your hand, then shuffle."
///
/// That filter is the whole card, so the library is built to tell it from the
/// two things it could have been: sixty Forests are no creature card at all, a
/// Llanowar Elves is a 1/1 that qualifies, and a Juzam Djinn is a 5/5 that does
/// not. The search offers exactly the Elves and the Djinn is still in the
/// library afterwards — a filter that had read `Filter::CREATURE` alone would
/// have offered both, and one that offered nothing would prove nothing.
#[test]
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
fn recruiter_of_the_guard_finds_the_cheap_creature_and_leaves_the_expensive_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(
            0,
            &[recruiter_of_the_guard(), llanowar_elves(), juzam_djinn()],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The two creature cards go into the library by hand, because `SeatSpec`
    // has no field for one: the 1/1 the filter is written for and the 5/5 it
    // must refuse. They come out of the opening hand, which is the only place
    // on this board where creature cards exist.
    let small = in_hand(&engine, p0, llanowar_elves()).expect("the 1/1 starts in hand");
    let big = in_hand(&engine, p0, juzam_djinn()).expect("the 5/5 starts in hand");
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        for card in [small, big] {
            state
                .move_object(
                    card,
                    ZoneLocation::Library(p0),
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .expect("the harness moves a card");
        }
    }
    engine.refresh_offer();

    let library_before = library_size(&engine, p0);
    cast_from_hand(&mut engine, p0, recruiter_of_the_guard());

    // The 1/1 resolves and its enters-ability asks; the `may` is answered on
    // the way, so the walk stops on the search itself.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched");
    };
    assert_eq!(player, p0, "the Recruiter's controller does the searching");
    assert_eq!(
        prompt,
        ChoicePrompt::SearchLibrary,
        "the tutor's own question, and not a scry or a discard"
    );
    assert_eq!(
        options.len(),
        1,
        "sixty Forests are no creature cards and one card in the library is a \
         creature with toughness 2 or less: {options:?}"
    );
    assert_eq!(
        engine
            .state()
            .object(options[0])
            .and_then(|o| o.card)
            .map(|c| c.index),
        Some(llanowar_elves()),
        "the 1/1 is the card on the menu, and not the 5/5 beside it"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("the card the search offered is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_some(),
        "\"put it into your hand\": the very card the search offered"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .contains(&big),
        "the 5/5 was never offered, so it never moved: the filter is a \
         toughness and not `Filter::CREATURE`"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert!(
        on_battlefield(&engine, p0, recruiter_of_the_guard()).is_some(),
        "and the Recruiter itself resolved onto the battlefield"
    );
}
