//! `cards/instants/mv_8/dig_through_time.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dig Through Time over a library of one puts that one into your hand.
///
/// "Look at the top seven cards of your library. Put two of them into your
/// hand and the rest on the bottom of your library in any order." With one
/// card left the effect does only as much as possible (CR 609.3): it puts the
/// one. It asked for two out of one instead, a question no answer could
/// satisfy, and the table stopped — the house's proposal and its fallback
/// both refused (r002 games 368 and 2675).
#[test]
fn dig_through_time_over_a_library_of_one_puts_that_one_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(17, island())
        .battlefield(0, &[island(); 8])
        .hand(0, &[dig_through_time()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let left = library_size(&engine, p0);
    seed_graveyard(&mut engine, p0, left - 1);
    let last = engine.state().zones.list(ZoneLocation::Library(p0))[0];
    cast_from_hand(&mut engine, p0, dig_through_time());
    // Delve offers the graveyard just filled; eight Islands pay without it.
    if let Pending::ChooseCards {
        prompt: ChoicePrompt::Delve,
        ..
    } = engine.pending()
    {
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
            .expect("delving nothing is an answer");
    }
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::PutIntoHand,
                ..
            }
        )
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(options, vec![last], "the one card there is");
    assert_eq!((min, max), (1, 1), "asked for as many as there are");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![last],
            },
        )
        .expect("the one card is the answer");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&last),
        "the card went to hand"
    );
    assert_eq!(library_size(&engine, p0), 0);
}

/// Dig Through Time cast from the graveyard (Snapcaster Mage's flashback)
/// cannot delve itself away.
///
/// CR 601.2a moves a spell to the stack before its costs are paid (601.2h),
/// so while delve (CR 702.66a) exiles cards from the graveyard to pay, the
/// spell is not one of them. The engine moves the card at the end of the
/// payment instead, and the delve question offered the whole graveyard, the
/// card being cast included: exiled for delve, it paid {1} of its own cost
/// and went on to the stack from exile.
#[test]
fn dig_through_time_flashed_back_does_not_offer_itself_to_its_own_delve() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(17, island())
        .battlefield(0, &[island(); 9])
        .hand(0, &[snapcaster_mage(), dig_through_time()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p0, 3);
    let dig = in_hand(&engine, p0, dig_through_time()).expect("the Dig in hand");
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .move_object(
            dig,
            ZoneLocation::Graveyard(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Cost,
        )
        .expect("the Dig goes to the graveyard");
    engine.refresh_offer();
    let dig = in_graveyard(&engine, p0, dig_through_time()).expect("the Dig in the graveyard");
    let two: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Battlefield)[..2].to_vec();
    tap_mana_where(&mut engine, p0, |id| two.contains(&id));
    cast_with_floating(&mut engine, p0, snapcaster_mage());
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseTargets { options, .. } if options.contains(&dig)),
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![dig],
                players: vec![],
            },
        )
        .expect("the Dig is Snapcaster's target");
    pass_until(&mut engine, stack_is_empty);
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: dig })
        .expect("the Dig has flashback");
    let Pending::ChooseCards {
        options,
        prompt: ChoicePrompt::Delve,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the delve question, got {:?}", engine.pending())
    };
    assert_eq!(options.len(), 3, "the three other cards: {options:?}");
    assert!(!options.contains(&dig), "the spell cannot pay for itself");
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: options })
        .expect("delving the other three is an answer");
    assert!(
        on_stack(&engine, dig_through_time()).is_some(),
        "the Dig is cast: {:?}",
        engine.pending()
    );
}

#[test]
fn dig_retains_both_selected_commanders_across_individual_replacement_answers() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[island(); 8])
        .hand(0, &[dig_through_time()])
        .commander(0, &[index::GRIZZLY_BEARS, index::SERRA_ANGEL])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let chosen: Vec<_> = e.state().commanders[0].iter().map(|c| c.object).collect();
    for &card in &chosen {
        e.dev_state_mut(p0)
            .unwrap()
            .move_object(
                card,
                ZoneLocation::Library(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    cast_from_hand(&mut e, p0, dig_through_time());
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::PutIntoHand,
                ..
            }
        )
    });
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: chosen.clone(),
        },
    )
    .unwrap();
    for (card, home) in [(chosen[0], true), (chosen[1], false)] {
        assert!(
            matches!(e.pending(),Pending::YesNo{prompt:crate::choice::YesNoPrompt::CommanderReplace{card:asked,to_library:false},..} if *asked==card)
        );
        // Neither card moves before both replacement answers are known.
        assert!(
            chosen
                .iter()
                .all(|c| e.state().object(*c).unwrap().zone == Zone::Library)
        );
        e.apply(p0, PlayerAction::YesNo(home)).unwrap();
    }
    let Pending::Arrange { cards, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert_eq!(cards.len(), 5);
    assert!(cards.iter().all(|c| !chosen.contains(c)));
    e.apply(p0, PlayerAction::Arrange { piles: vec![cards] })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(chosen[0]).unwrap().zone, Zone::Command);
    assert_eq!(e.state().object(chosen[1]).unwrap().zone, Zone::Hand);
}
