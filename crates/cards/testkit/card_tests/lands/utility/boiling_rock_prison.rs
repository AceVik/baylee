//! `cards/lands/utility/boiling_rock_prison.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Boiling Rock Prison prints three lines: it enters tapped, it taps for {B}
/// or {R}, and it will sell itself and four mana for a card. An enters-tapped
/// land is the one thing a *seated* board cannot show — `starting_battlefield`
/// moves a card without an entry, so a permanent placed there arrives
/// untapped — which is why the land is played from hand, and the untap step a
/// cycle later is the control for the status the drop left it in. The cheaper
/// line is then read as the question it is, two colours and no third, and the
/// `{4}` line is only claimed once the four mana are really floating:
/// `legal.abilities` is filtered through `can_afford`, which reads the pool
/// rather than the lands beside it.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn boiling_rock_prison_enters_tapped_sells_black_or_red_and_trades_itself_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[boiling_rock_prison()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The printed entry, read off a real land drop: the card comes out of the
    // hand rather than being seated, so whatever taps it on the way in has an
    // entry to look at.
    play_land(&mut engine, p0, boiling_rock_prison());
    let prison = on_battlefield(&engine, p0, boiling_rock_prison())
        .expect("the played land is on the battlefield");
    assert!(
        entered_tapped(&engine, prison),
        "\"This land enters tapped\" — a `starting_battlefield` placement would \
         have arrived untapped, which is why the drop is a real one"
    );

    // The untap step, and the control for the status above: the land stands
    // back up, so the tap symbol it prints is payable.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, prison),
        "the untap step stood the land back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool is empty: nothing on this board has been tapped yet"
    );

    // "{T}: Add {B} or {R}" — one mana of one of two colours, so the engine
    // asks which, and the list is the card's own pair.
    activate(&mut engine, p0, boiling_rock_prison(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}} or {{R}}\" is a question, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        2,
        "the two colours the card prints, and no third: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Red),
        "both halves of \"or\" are on the menu: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Black),
        0,
        "\"or\" is one mana of one colour, not both"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, prison), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );

    // The tap is spent, so the same line is no longer one the seat may take.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.contains(&(prison, 0)),
        "a tapped land has no {{T}} left to pay its mana line with: {:?}",
        legal.abilities
    );

    // The third line charges the tap as well, so it needs the turn cycle back.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, prison), "the untap step again");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the red it made emptied with the step that ended (CR 500.5)"
    );

    // The land is named as the printing kept back: it prints a
    // `{T}: Add {B} or {R}` of its own, so `tap_all_mana` would have spent the
    // very permanent whose {{T}} this activation charges (#159).
    tap_all_mana_but(&mut engine, p0, Some(boiling_rock_prison()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Mountains, four mana: exactly the {{4}} the ability charges"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(prison, 1)),
        "with {{4}} in the pool the second printed line is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, boiling_rock_prison(), 1);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{4}} came out of the pool"
    );
    assert!(
        on_battlefield(&engine, p0, boiling_rock_prison()).is_none(),
        "`Sacrifice this land` is part of the price and is paid on announcement"
    );
    assert!(
        in_graveyard(&engine, p0, boiling_rock_prison()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it reached the hand, so an emptied library would not satisfy the \
         count above"
    );
}
