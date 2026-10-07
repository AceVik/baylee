//! `cards/lands/utility/misty_palms_oasis.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Misty Palms Oasis prints three lines and this scenario plays all of them:
/// it enters tapped, it taps for {W} or {B}, and {4}, {T}, Sacrifice it draws
/// a card. The land played from hand is what proves the entry — a permanent
/// that came down tapped offers nothing, because `{T}` is the whole price of
/// its mana line — while a second copy seated on the battlefield untapped is
/// the one whose abilities can be read off the same board, the mana line as
/// the two-colour question the card prints and the draw line as the four
/// mana, the tap and the land itself, each landing in a zone a test can count.
#[test]
#[allow(clippy::too_many_lines)]
fn misty_palms_oasis_enters_tapped_and_sells_itself_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), misty_palms_oasis()],
        )
        .hand(0, &[misty_palms_oasis()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let seated =
        on_battlefield(&engine, p0, misty_palms_oasis()).expect("the seated Oasis is on the table");
    let played = play_land(&mut engine, p0, misty_palms_oasis());
    assert_ne!(played, seated, "the played land is a second card");
    assert!(
        is_tapped(&engine, played),
        "\"This land enters tapped\" is an entry and not a placement"
    );
    assert!(
        !is_tapped(&engine, seated),
        "the seated copy was placed by the harness, so no enter modifier \
         looked at it"
    );

    // Both printed abilities pay the `{T}` symbol, and the mana line pays
    // nothing else — so the land that just came down is offered neither of
    // them, while the untapped copy of the same card standing beside it is.
    // That neighbour is the control: the absence is the tap, not a board the
    // engine declined to read.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(seated, 0)),
        "an untapped Oasis is a paid {{T}}, so its mana line is offered: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(played, 0)),
        "and the tapped one has no {{T}} left to pay with, which costs that \
         line nothing but the tap: {:?}",
        legal.abilities
    );

    // `{4}` is read off the pool and not off the untapped lands, so the mana
    // has to be floating before anything is claimed about the offer. The two
    // Oases are named as the printing kept back: the seated one is the land
    // this turn is about, and the played one has its own turn below.
    tap_all_mana_but(&mut engine, p0, Some(misty_palms_oasis()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests, four green, and neither Oasis contributed"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(seated, 0)) && legal.abilities.contains(&(seated, 1)),
        "with {{4}} floating both printed lines are payable: {:?}",
        legal.abilities
    );

    // The mana line is the choice itself: two printed colours and no third.
    activate(&mut engine, p0, misty_palms_oasis(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{W}} or {{B}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options,
        vec![ManaColor::White, ManaColor::Black],
        "the two colours the card prints, and colourless is no colour at all \
         (CR 105.4)"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white was one of the colours it offered");
    assert!(
        is_tapped(&engine, seated),
        "the tap symbol was the whole price of that line"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        5,
        "the four Forests and the one mana off the Oasis"
    );

    // Across the opponent's turn and back: a pool empties at the end of a step
    // (CR 500.5), and a land that came down tapped has no `{T}` until its
    // controller's untap step.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, played) && !is_tapped(&engine, seated),
        "the untap step stood both lands back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the white the Oasis made is gone with the turn it was made in"
    );

    tap_all_mana_but(&mut engine, p0, Some(misty_palms_oasis()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests again, and the two untapped Oases make no mana of their own"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(played, 1)),
        "the line this turn is about: {{4}}, {{T}}, Sacrifice this land: Draw \
         a card: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: played,
                ability_index: 1,
            },
        )
        .expect("the four mana already floating pay the whole price");

    assert!(
        in_graveyard(&engine, p0, misty_palms_oasis()).is_some(),
        "sacrificing the land is part of the price and is paid on announcement"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{4}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the ability is waiting on the stack"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the card arrives on resolution and not on announcement"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it is in hand, so an emptied library would not satisfy the count above"
    );
    assert!(
        on_battlefield(&engine, p0, misty_palms_oasis()).is_some(),
        "the copy the ability did not name is still on the battlefield"
    );
}
