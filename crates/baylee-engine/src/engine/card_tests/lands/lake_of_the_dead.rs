//! `cards/lands/lake_of_the_dead.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lake of the Dead is `Coverage::Partial`: the entry replacement (sacrifice a
/// Swamp or else put this into the graveyard) has no `EnterModifier` and is
/// not implemented — the land enters normally.  What is implemented: `{T}: Add
/// {B}` and `{T}, Sacrifice a Swamp: Add {B}{B}{B}{B}`.
///
/// The second line is the notable one: its cost asks which Swamp to sacrifice,
/// and the engine has to offer the question rather than refuse the activation
/// for want of a named cost target.  A Swamp under the same seat is the only
/// legal answer; the controller's own Island and a Swamp across the table are
/// both rejected.  After the sacrifice the pool holds `{B}{B}{B}{B}` and the
/// Stack is empty (mana ability, CR 605.3b).
#[test]
#[allow(clippy::too_many_lines)] // one card, four printed claims
fn lake_of_the_dead_sacrifices_a_swamp_for_four_black() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(61, swamp())
        .battlefield(
            0,
            &[
                lake_of_the_dead(),
                swamp(),  // the fodder
                island(), // a non-Swamp: should not be on the list
            ],
        )
        .battlefield(1, &[swamp()]) // their Swamp: not yours to sacrifice
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lake = on_battlefield(&engine, p0, lake_of_the_dead()).expect("the Lake is out");
    let fodder = on_battlefield(&engine, p0, swamp()).expect("my Swamp is out");
    let non_swamp = on_battlefield(&engine, p0, island()).expect("my Island is out");
    let theirs = on_battlefield(&engine, p1, swamp()).expect("their Swamp is out");

    // Nothing is tapped for mana first, and that is the point: the line's
    // whole cost is `{T}, Sacrifice a Swamp`, so an empty pool is what makes
    // "four black and nothing else" an exact reading of the card rather
    // than a delta. `tap_mana_except` left the Island and the fodder Swamp
    // floating and the assertion below then counted five.

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    // The Lake's sacrifice line is a printed mana ability → legal.abilities.
    assert!(
        legal.abilities.contains(&(lake, 1)),
        "the sacrifice line is offered: {:?}",
        legal.abilities
    );

    // Activate ability index 1 ({T}, Sacrifice a Swamp: Add {B}{B}{B}{B}).
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: lake,
                ability_index: 1,
            },
        )
        .expect("the cost asks which Swamp to sacrifice");

    // The cost asks which Swamp.
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected ChooseCards for the Swamp sacrifice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is asked");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost, not a search"
    );
    assert_eq!((min, max), (1, 1), "exactly one Swamp");
    assert!(
        options.contains(&fodder),
        "my Swamp is on the list: {options:?}"
    );
    assert!(
        !options.contains(&non_swamp),
        "the Island is not a Swamp: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "CR 701.21a: sacrifice only what you control: {options:?}"
    );
    assert!(
        !options.contains(&lake),
        "the Lake is not a Swamp: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the Swamp on the list pays the cost");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        4,
        "{{T}}, Sacrifice a Swamp: Add {{B}}{{B}}{{B}}{{B}}"
    );
    assert_eq!(pool.total(), 4, "and nothing else");
    assert!(
        stack_is_empty(&engine),
        "a mana ability never uses the stack (CR 605.3b)"
    );
    assert!(
        on_battlefield(&engine, p0, swamp()).is_none(),
        "the Swamp was sacrificed and left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, swamp()).is_some(),
        "the sacrificed permanent is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, swamp()).is_some(),
        "the opponent's Swamp was never touched"
    );
}
