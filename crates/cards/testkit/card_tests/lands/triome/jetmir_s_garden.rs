//! `cards/lands/triome/jetmir_s_garden.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Jetmir's Garden is a triome: `Land — Mountain Forest Plains` that enters
/// tapped, taps for one mana of any of its three types' colours, and cycles for
/// `{3}` out of hand. One game reads all three printed lines, and each needs its
/// own reading: the land is *played* rather than seated, because a permanent the
/// harness places never exercises an entry modifier; the same object offers
/// nothing while it is down and its three colours once the untap step has stood
/// it up; and a second copy is cycled so the discard, the draw and the `{3}` each
/// land in a zone a test can see.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn jetmirs_garden_enters_tapped_then_makes_one_of_its_three_colors_and_cycles_for_three() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[jetmirs_garden(), jetmirs_garden(), silence()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // "This land enters tapped" is a real entry and not a placement: the
    // harness' opening battlefield would have put it down untapped whatever the
    // card says, so the land drop is what is played here.
    let garden = play_land(&mut engine, p0, jetmirs_garden());
    assert!(
        entered_tapped(&engine, garden),
        "\"This land enters tapped.\""
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and a land is no mana: nothing was tapped to play it"
    );

    // The negative half of the mana line, read on the very object that will
    // offer it in a moment: a land that arrived tapped has no {{T}} left to pay
    // with, so neither the type line's intrinsic source nor a printed one is
    // named in the offer.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the land's controller holds priority");
    assert!(
        !legal.mana_abilities.contains(&garden)
            && !legal.abilities.iter().any(|(id, _)| *id == garden),
        "a land that arrived tapped is no mana source this turn: {legal:?}"
    );

    // "Cycling {3}" — {3}, Discard this card: Draw a card. The three Forests are
    // the whole price, and the offer is filtered through `can_afford`, which
    // reads the pool rather than the untapped lands: so the mana goes in first.
    let spare = in_hand(&engine, p0, jetmirs_garden()).expect("the second copy is in hand");
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(
        taken, 3,
        "three Forests and nothing else on this board makes mana"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three green, which is the cycling cost exactly"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == spare)
        .expect("cycling is an activated ability of the card in hand (CR 702.29a)");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the three green already floating are the {{3}}");

    // CR 601.2h: the discard is a cost and is paid as the ability is announced,
    // while the draw is the effect and waits on the stack.
    assert!(
        in_graveyard(&engine, p0, jetmirs_garden()).is_some(),
        "\"Discard this card\" is half the price and lands in the graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the other half was the {{3}} that was floating"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the ability uses the stack"
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
        "one discarded and one drawn, so the hand is the size it was"
    );
    assert!(
        on_battlefield(&engine, p0, jetmirs_garden()).is_some(),
        "and the copy that was played is untouched: cycling is a hand ability"
    );

    // A turn round the table, because a land that entered tapped gives nothing
    // until its controller's own untap step (CR 502.3).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, garden),
        "the untap step stood the Garden back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "and it is the seat whose land came back up");
    // A land whose basic land types grant it mana is the CR 305.6 shortcut, and
    // a card that prints its own "{T}: Add …" is an ordinary indexed ability;
    // this triome is written both ways, so the offer is read, not guessed.
    let action = if legal.mana_abilities.contains(&garden) {
        PlayerAction::ActivateManaAbility { source: garden }
    } else {
        let (source, ability_index) = legal
            .abilities
            .iter()
            .copied()
            .find(|(id, _)| *id == garden)
            .expect("an untapped Jetmir's Garden is a mana source in one of the two lists");
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        }
    };
    engine
        .apply(p0, action)
        .expect("the offer named this source, and its whole price is its own {{T}}");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{R}}, {{G}}, or {{W}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped it names the colour");
    assert_eq!(
        options.len(),
        3,
        "the three colours the type line prints, and no fourth: {options:?}"
    );
    for color in [ManaColor::Red, ManaColor::Green, ManaColor::White] {
        assert!(
            options.contains(&color),
            "\"...{{R}}, {{G}}, or {{W}}\" includes {color:?}: {options:?}"
        );
    }
    assert!(
        !options.contains(&ManaColor::Blue) && !options.contains(&ManaColor::Black),
        "and neither colour it does not print, which \"any color\" would have offered: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the three Forests are standing untapped and make green besides, so the \
         white has no other source on this board"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert!(
        is_tapped(&engine, garden),
        "and the Garden paid its own {{T}}"
    );

    // The colour is not a label: the one mana pays for a spell that costs {{W}},
    // which nothing else on this board could have produced.
    cast_with_floating(&mut engine, p0, silence());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, silence()).is_some(),
        "the {{W}} was spent on a white spell rather than sitting in the pool as a name"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and paying for it emptied the pool the Garden filled"
    );
}
