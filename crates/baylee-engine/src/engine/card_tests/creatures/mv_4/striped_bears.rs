//! `cards/creatures/mv_4/striped_bears.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Striped Bears — {3}{G}, a 2/2 Bear: "When this creature enters, draw a
/// card." The body and the draw are read off the same cast, and each needs a
/// different place on the board: the 2/2 on the battlefield says the spell
/// resolved, and the hand count says the card that left it to get there is
/// the one that arrived. The draw is walked past in two steps rather than one
/// — the permanent lands with its trigger still on the stack (CR 603.3b), so
/// the library is untouched at that moment and exactly one card shorter once
/// the stack empties, which is a trigger and not the spell's own resolution.
#[test]
fn striped_bears_draws_a_card_on_the_way_in() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[striped_bears()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Four Forests are exactly the {3}{G} the card costs, and the offer is
    // read with the mana already floating: `can_afford` reads the pool and
    // not the untapped lands.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests tapped, four green"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let card = in_hand(&engine, p0, striped_bears()).expect("the Bears are in hand");
    assert!(
        legal.castable.contains(&card),
        "{{3}}{{G}} is payable out of the pool the Forests just filled: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, striped_bears());

    // The permanent arrives first and asks its question second: stop where
    // the Bears is on the battlefield and its enters ability is still on the
    // stack. The hand is one card short there — the Bears left it and nothing
    // has come back — and the library is exactly as it was.
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, striped_bears()).is_some() && !stack_is_empty(e)
    });
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "nothing has been drawn while the trigger is still on the stack"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "and the hand is one short, because the Bears moved rather than was copied"
    );

    pass_until(&mut engine, stack_is_empty);

    let bears = on_battlefield(&engine, p0, striped_bears()).expect("the Bears resolved");
    let body = engine
        .state()
        .object(bears)
        .expect("the Bears are still an object")
        .characteristics();
    assert!(
        body.types.contains(TypeSet::CREATURE),
        "a Creature, and not merely a card that changed zones: {:?}",
        body.types
    );
    assert_eq!(pt(&engine, bears), (2, 2), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{G}} came out of the pool"
    );

    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"when this creature enters, draw a card\" — exactly one card off the \
         top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "and the hand is back where it started, which it can only be because \
         the Bears left it and a drawn card arrived: a trigger that drew \
         nothing would leave it at {}",
        hand_before - 1
    );
    assert!(
        in_hand(&engine, p0, striped_bears()).is_none(),
        "the card missing from the hand is the one standing on the \
         battlefield, not a copy drawn back into it"
    );
}

/// "When this creature enters, it deals 4 damage divided as you choose among
/// any number of target creatures and/or planeswalkers." Three targets: the
/// division is announced with the targets (CR 601.2d), target by target,
/// each at least 1 and never so much that a later one is left none; the last
/// takes what is left without being asked.
#[test]
fn fury_divides_four_damage_among_three_targets() {
    let p1 = PlayerId::new(1);
    let mut engine = fury_enters(
        &[thundering_giant(), striped_bears(), llanowar_elves()],
        false,
    );
    let giant = on_battlefield(&engine, p1, thundering_giant()).unwrap();
    let bears = on_battlefield(&engine, p1, striped_bears()).unwrap();
    let elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    fury_aims(&mut engine, vec![giant, bears, elves]);

    assert_eq!(
        fury_share(&engine, giant, 0, 3, 4),
        (1, 2),
        "two more targets still need 1 each"
    );
    assert!(
        engine
            .apply(PlayerId::new(0), PlayerAction::ChooseNumber(3))
            .is_err(),
        "3 would leave one target without damage"
    );
    engine
        .apply(PlayerId::new(0), PlayerAction::ChooseNumber(2))
        .unwrap();
    assert_eq!(fury_share(&engine, bears, 1, 3, 2), (1, 1));
    engine
        .apply(PlayerId::new(0), PlayerAction::ChooseNumber(1))
        .unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "the last target takes the rest unasked, got {:?}",
        engine.pending()
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(marked(&engine, giant), 2, "the Giant's share");
    assert_eq!(marked(&engine, bears), 1, "the Bears' share");
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the Elves' 1 was lethal"
    );
}

/// Two targets: the first is dealt what its controller names and the
/// second the whole of the rest — 3 here, which kills the Bears.
#[test]
fn fury_gives_the_last_target_the_rest() {
    let p1 = PlayerId::new(1);
    let mut engine = fury_enters(&[thundering_giant(), striped_bears()], false);
    let giant = on_battlefield(&engine, p1, thundering_giant()).unwrap();
    let bears = on_battlefield(&engine, p1, striped_bears()).unwrap();
    fury_aims(&mut engine, vec![giant, bears]);
    assert_eq!(fury_share(&engine, giant, 0, 2, 4), (1, 3));
    engine
        .apply(PlayerId::new(0), PlayerAction::ChooseNumber(1))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(marked(&engine, giant), 1);
    assert!(
        in_graveyard(&engine, p1, striped_bears()).is_some(),
        "the Bears were dealt the other 3"
    );
}
