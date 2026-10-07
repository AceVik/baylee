//! `cards/creatures/mv_3/taoist_hermit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Taoist Hermit — {2}{G} 2/2 Human Mystic with hexproof (CR 702.11b):
/// "This creature can't be the target of spells or abilities your
/// opponents control."
///
/// Hexproof is half a rule and the board plays both halves in one game. The
/// Hermit has to be a legal target for its *own* controller's Giant Growth —
/// the +3/+3 is read straight off the projected body afterwards — while the
/// opponent's Vindicate, cast in their own main phase, has to offer the
/// Llanowar Elves standing beside it and refuse the Hermit. The Elves is the
/// control that keeps the refusal from being an empty menu: same controller,
/// same card type, no hexproof, so a menu that reached it and not the Hermit
/// is the keyword's doing and not a broken offer.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn taoist_hermit_is_its_own_controllers_target_and_never_the_opponents() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), llanowar_elves()],
        )
        .hand(0, &[taoist_hermit(), giant_growth()])
        // Exactly the {1}{W}{B} the Vindicate charges: two Plains and a Swamp.
        .battlefield(1, &[plains(), plains(), swamp()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Four Forests pay the Hermit's {2}{G} and leave the {G} the Giant Growth
    // is about to charge. The Elves is named as the printing kept back: it is
    // the creature the opponent's removal is read against, and a mana creature
    // tapped for the Hermit would be a target that had already moved for a
    // reason of its own.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests, and the Elves' own {{T}} left alone"
    );
    cast_with_floating(&mut engine, p0, taoist_hermit());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let hermit = on_battlefield(&engine, p0, taoist_hermit()).expect("the Hermit resolved");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    assert_eq!(pt(&engine, hermit), (2, 2), "the body the card prints");
    assert!(
        keywords(&engine, hermit).contains(KeywordSet::HEXPROOF),
        "the printed keyword reaches the permanent through the layers"
    );

    // The half that is the controller's: hexproof is the *opponents'* word,
    // so p0's own spell may still name it (CR 702.11b).
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{2}}{{G}} is spent and the Giant Growth's {{G}} is still floating"
    );
    cast_with_floating(&mut engine, p0, giant_growth());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it aims it");
    assert!(
        options.contains(&hermit),
        "its own controller's spell is still allowed to name it: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![hermit],
            },
        )
        .expect("the Hermit was one of the options the spell enumerated");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        pt(&engine, hermit),
        (5, 5),
        "the pump resolved, so the offer was a real target and not a filtered one"
    );

    // The other side of the table, and the same spell pointed at a creature
    // with no keyword on it at all.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p1, "the opponent aims their own removal");
    assert!(
        options.contains(&elves),
        "the creature beside it is the control — same controller, same card \
         type, no hexproof — so the offer does reach this seat's board: {options:?}"
    );
    assert!(
        !options.contains(&hermit),
        "\"target permanent\" is every permanent on the table except the one \
         hexproof refuses an opponent: {options:?}"
    );

    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("an option the offer enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the removal resolved against the target it was allowed"
    );
    assert!(
        on_battlefield(&engine, p0, taoist_hermit()).is_some(),
        "and the hexproof creature is exactly where it was"
    );
}
