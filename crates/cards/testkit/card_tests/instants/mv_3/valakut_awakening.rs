//! `cards/instants/mv_3/valakut_awakening.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Valakut Stoneforge, the back face: "This land enters tapped. {T}: Add
/// {R}." The test plays the back face as a land, confirms it enters tapped,
/// advances to the next turn so it untaps, and activates its mana ability to
/// add `{R}`.
#[test]
fn valakut_stoneforge_enters_tapped_and_taps_for_red_mana() {
    let (mut engine, stoneforge) =
        play_land_face(valakut_awakening(), 1).expect("plays as Valakut Stoneforge");
    assert!(
        is_tapped(&engine, stoneforge),
        "Valakut Stoneforge enters tapped"
    );

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, stoneforge), "untaps on next turn");
    let red_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Red);

    activate(&mut engine, p0, valakut_awakening(), 0);

    assert!(
        is_tapped(&engine, stoneforge),
        "tapped to activate mana ability"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        red_before + 1,
        "adds one red mana to the pool"
    );
}

/// Casts Valakut Awakening's front face from three Mountains and lets it
/// ask its question. Returns the menu of hand cards the question offered
/// (`min`, `max` and the options), answered with `put` -- the cards named by
/// index, in whatever order they sit in the hand.
#[track_caller]
fn cast_and_put_on_the_bottom(
    engine: &mut Engine<RegistryLookup>,
    p0: PlayerId,
    put: &[CardIndex],
) -> Option<(u8, u8, Vec<ObjectId>)> {
    tap_all_mana(engine, p0);
    cast_front_face(engine, p0, valakut_awakening());
    pass_until(engine, |e| {
        stack_is_empty(e) || matches!(e.pending(), Pending::ChooseCards { .. })
    });
    if stack_is_empty(engine) {
        return None;
    }
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on a card choice")
    };
    assert_eq!(player, p0, "the caster chooses");
    let chosen: Vec<ObjectId> = put
        .iter()
        .map(|card| in_hand(engine, p0, *card).expect("the card is in hand"))
        .collect();
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: chosen })
        .expect("a subset of the hand is a legal answer");
    pass_until(engine, stack_is_empty);
    Some((min, max, options))
}

/// "Put any number of cards from your hand on the bottom of your library,
/// then draw that many cards plus one." Two of the three other hand cards go
/// to the bottom, three cards are drawn, and the question's menu is exactly
/// the hand -- any number, none included, up to all of it.
#[test]
fn valakut_awakening_puts_cards_on_the_bottom_and_draws_that_many_plus_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[valakut_awakening(), forest(), plains(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let forest_card = in_hand(&engine, p0, forest()).expect("a Forest in hand");
    let plains_card = in_hand(&engine, p0, plains()).expect("a Plains in hand");
    let island_card = in_hand(&engine, p0, island()).expect("an Island in hand");
    let library_before = library_size(&engine, p0);

    let (min, max, options) =
        cast_and_put_on_the_bottom(&mut engine, p0, &[forest(), plains()]).expect("it asked");

    assert_eq!((min, max), (0, 3), "any number: none up to the whole hand");
    let mut menu = options;
    menu.sort();
    let mut hand = vec![forest_card, plains_card, island_card];
    hand.sort();
    assert_eq!(menu, hand, "the menu is the hand, and not the spell itself");
    let hand_now = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();
    assert_eq!(
        hand_now.len(),
        4,
        "the Island stayed, and two plus one cards were drawn: {hand_now:?}"
    );
    assert!(
        in_hand(&engine, p0, island()).is_some(),
        "the Island stayed"
    );
    assert!(
        in_hand(&engine, p0, forest()).is_none() && in_hand(&engine, p0, plains()).is_none(),
        "the two put away are not in hand -- and were not drawn back"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, mountain()).len(),
        3,
        "the lands paid and stayed"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before + 2 - 3,
        "two cards in, three out"
    );
    let mut bottom: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .iter()
        .copied()
        .take(2)
        .collect();
    bottom.sort();
    let mut put = vec![forest_card, plains_card];
    put.sort();
    assert_eq!(
        bottom, put,
        "the bottom of the library is exactly the two cards put there"
    );
}

/// "If you put zero cards on the bottom of your library, you draw one card."
/// (ruling, 2020-09-25.) Hand cards are on offer and the player names none.
#[test]
fn valakut_awakening_putting_nothing_back_draws_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[valakut_awakening(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let library_before = library_size(&engine, p0);

    let (min, max, _) = cast_and_put_on_the_bottom(&mut engine, p0, &[]).expect("it asked");

    assert_eq!((min, max), (0, 1), "naming none is a legal answer");
    assert!(
        in_hand(&engine, p0, forest()).is_some(),
        "the Forest stayed"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        2,
        "the Forest and exactly one drawn card"
    );
    assert_eq!(library_size(&engine, p0), library_before - 1);
}

/// "Any number" includes all of them: with the whole hand put away, the
/// player draws that many cards plus one.
#[test]
fn valakut_awakening_putting_the_whole_hand_back_draws_one_more_than_it_held() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[valakut_awakening(), forest(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let library_before = library_size(&engine, p0);

    cast_and_put_on_the_bottom(&mut engine, p0, &[forest(), plains()]).expect("it asked");

    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();
    assert_eq!(hand.len(), 3, "2 put away, so 2 + 1 drawn: {hand:?}");
    assert!(
        in_hand(&engine, p0, forest()).is_none() && in_hand(&engine, p0, plains()).is_none(),
        "all three drawn cards came off the top, not off the bottom they were put on"
    );
    assert_eq!(library_size(&engine, p0), library_before - 1);
}

/// With nothing else in hand there is nothing to put on the bottom and no
/// question to ask: the spell still draws its one card.
#[test]
fn valakut_awakening_with_an_empty_hand_still_draws_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[valakut_awakening()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let library_before = library_size(&engine, p0);

    let asked = cast_and_put_on_the_bottom(&mut engine, p0, &[]);

    assert!(asked.is_none(), "an empty hand has nothing to offer");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "zero put back, plus one"
    );
    assert_eq!(library_size(&engine, p0), library_before - 1);
}
