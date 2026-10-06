//! `cards/creatures/mv_3/bog_witch.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bog Witch is a {2}{B} 1/1 printing `{B}, {T}, Discard a card: Add
/// {B}{B}{B}` — a *mana* ability (CR 605.1a), so all three printed parts of
/// the price have to be paid and the three black arrive with nothing on the
/// stack (CR 605.3b). Four Swamps pay the {2}{B} to cast her and leave
/// exactly the {B} the ability charges still floating in the same main phase
/// (CR 500.5), so the offer can only be read off the pool. The discard is a
/// cost and not a search, and the leftover {B} answers the price exactly:
/// one black spent, three returned, nothing else on the board.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn bog_witch_taps_discards_and_adds_three_black_without_using_the_stack() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(917, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[bog_witch(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four Swamps, and the Witch is still in hand: `tap_all_mana` cannot press
    // her own `{B}, {T}, Discard a card` line because she is not a permanent
    // yet, so the cast takes {2}{B} out of the four and leaves the {B} the
    // ability charges floating beside it.
    cast_from_hand(&mut engine, p0, bog_witch());
    pass_until(&mut engine, stack_is_empty);
    let witch = on_battlefield(&engine, p0, bog_witch()).expect("the Witch resolved");
    assert!(!is_tapped(&engine, witch), "she arrives untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "four Swamps less the {{2}}{{B}} the Witch cost"
    );

    // And with the mana right there she is still not offered, because the
    // price has a `{T}` in it and she arrived this turn (CR 302.6). That is
    // the assertion the turn cross below is *for*: without it, "the line is
    // offered" would have been read off a board where it could not be.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(witch, 0)),
        "summoning sickness refuses the {{T}}: {:?}",
        legal.abilities
    );

    // The opponent's turn first, and only then back: `walk_to_own_main`
    // alone returns where it stands, because this *is* p0's own main phase.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the next own main phase, where she is no longer newly arrived"
    );
    let swamps = all_on_battlefield(&engine, p0, swamp());
    let one = swamps[0];
    tap_mana_where(&mut engine, p0, |id| id == one);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "exactly one Swamp tapped, which is the {{B}} the line charges"
    );

    // The price is a mana *and* the tap *and* a card, so the line is an
    // ordinary entry in `abilities` — a printed mana ability carries an index,
    // unlike the CR 305.6 shortcut a basic land uses.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(witch, 0)),
        "with {{B}} floating the whole cost is payable, so the one line she \
         prints is offered: {:?}",
        legal.abilities
    );
    assert!(
        !legal.mana_abilities.contains(&witch),
        "a printed mana ability is never the no-index shortcut: {:?}",
        legal.mana_abilities
    );

    let graveyard_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, bog_witch(), 0);
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
            "`Discard a card` is a cost and is asked before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostDiscard,
        "a cost and not a search, which is all a client has to tell them apart"
    );
    assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
    assert_eq!(
        options.len(),
        hand_before,
        "the whole hand is the menu and the battlefield is not: {options:?}"
    );
    assert!(
        !options.contains(&witch),
        "the Witch is a permanent on the battlefield, not a card in hand: {options:?}"
    );

    let discarded = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![discarded],
            },
        )
        .expect("a card the question offered is a legal answer");

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "and the seat holds priority again, got {:?}",
        engine.pending()
    );
    assert!(
        is_tapped(&engine, witch),
        "{{T}} is half the price she printed"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        3,
        "{{B}}{{B}}{{B}}: the one floating {{B}} paid the price and three came back"
    );
    assert_eq!(
        pool.total(),
        3,
        "and nothing beside it — the tapped Swamps are spent, not still floating"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .contains(&discarded),
        "the discarded card is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        graveyard_before + 1,
        "one card given up and none in beside it"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "the hand is one shorter, which a card merely revealed would not do"
    );
    assert!(
        on_battlefield(&engine, p0, bog_witch()).is_some(),
        "the tap is the whole of what the Witch herself gives up"
    );
}
