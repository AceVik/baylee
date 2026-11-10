//! `cards/lands/triome/xander_s_lounge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Xander's Lounge is a basic-typed tri-land — Island Swamp Mountain — that
/// enters tapped, taps for one mana of `{U}`, `{B}` or `{R}`, and cycles for
/// `{3}` out of hand. All three printed lines are played on one board, because
/// each is the other's control: the land arrives through a real land drop
/// (`starting_battlefield` would place it with `Cause::Setup`, and no
/// replacement effect ever looks at a placement, so a seeded copy would stand
/// untapped), the *second* copy is the one cycled so the discard costs a card
/// other than the permanent under test, and the mana ability is read a turn
/// later, where the untap step has given the same object its `{T}` back.
#[test]
#[allow(clippy::too_many_lines)] // one card, three printed lines, and the turn they need
fn xanders_lounge_enters_tapped_cycles_itself_and_taps_for_one_of_three_colors() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[xanders_lounge(), xanders_lounge()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let played = play_land(&mut engine, p0, xanders_lounge());
    assert!(
        entered_tapped(&engine, played),
        "\"This land enters tapped\" — and a real land drop is the only way to \
         see it, since a placement is not an entry"
    );
    let hand_copy = in_hand(&engine, p0, xanders_lounge()).expect("the second copy is in hand");
    assert_ne!(
        hand_copy, played,
        "and the copy the cycle will spend is not the permanent on the table"
    );

    // A tapped permanent has no `{T}` to pay with, so nothing on it is offered
    // at all — either way the engine could have named that tap. The same
    // object is offered its mana ability on the next turn, below.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.mana_abilities.contains(&played)
            && !legal.abilities.iter().any(|(id, _)| *id == played),
        "the land just played is tapped, so its {{T}} is unpayable and no \
         ability on it is offered: {:?}",
        legal.abilities
    );

    // Cycling `{3}` is the second ability the card prints: the mana ability is
    // index 0 and is a battlefield ability, so index 1 is the only entry the
    // hand can offer. The `{3}` is put into the pool first, because
    // `can_afford` reads the pool and not the three untapped Forests.
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests tapped for exactly the {{3}} the cycle charges, and the \
         played land makes nothing while it lies tapped"
    );

    activate(&mut engine, p0, xanders_lounge(), 1);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}} is a cost and is paid on announcement (CR 601.2h)"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "\"Discard this card\" — the cycled copy left the hand before the draw"
    );
    assert!(
        in_graveyard(&engine, p0, xanders_lounge()).is_some(),
        "and a card discarded to pay a cost goes to its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "the draw is no mana ability, so it is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "and it is in hand, so the discard and the draw net out to the hand the \
         cycle started with"
    );
    assert!(
        in_hand(&engine, p0, xanders_lounge()).is_none(),
        "no copy of the cycled card is left in hand"
    );
    assert!(
        on_battlefield(&engine, p0, xanders_lounge()).is_some(),
        "and cycling the other copy never touched the permanent on the table"
    );

    // The third printed line needs the untap step: a land that entered tapped
    // stands back up in its controller's untap step like anything else, and
    // only then is `{T}` a price it can pay.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, played),
        "the untap step stood the land back up, which is the whole reason the \
         ability below is payable"
    );

    // Ability 0 is the printed "{T}: Add {U}, {B}, or {R}" — one mana of one
    // of the three, asked as a question and not defaulted to a colour. Which
    // of the two lists the engine names that tap in is not this test's claim,
    // so the route is taken out of the offer rather than assumed.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let route = if legal.abilities.contains(&(played, 0)) {
        PlayerAction::ActivateAbility {
            source: played,
            ability_index: 0,
        }
    } else {
        assert!(
            legal.mana_abilities.contains(&played),
            "an untapped tri-land is a paid {{T}}, so its mana ability is \
             offered somewhere: {:?} / {:?}",
            legal.abilities,
            legal.mana_abilities
        );
        PlayerAction::ActivateManaAbility { source: played }
    };
    engine
        .apply(p0, route)
        .expect("the route came out of the list that offered it");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{U}}, {{B}}, or {{R}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options,
        vec![ManaColor::Blue, ManaColor::Black, ManaColor::Red],
        "the three colours the card prints, and colourless is no colour at all \
         (CR 105.4)"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the colours it offered");

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert!(is_tapped(&engine, played), "the land paid its own {{T}}");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Blue) + pool.available(ManaColor::Black),
        0,
        "\"or\" is one colour: the other two halves of the choice were not \
         added beside it"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana off one tap, with the green that paid the cycle long gone"
    );
}
