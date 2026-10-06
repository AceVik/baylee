//! `cards/lands/triome/ketria_triome.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ketria Triome is a Forest, Island and Mountain that arrives tapped, taps for
/// one mana of exactly those three colours, and cycles itself out of a hand for
/// `{3}`. One game plays all three sentences, because each is the other's
/// control: the turn cycle shows the tap was the printed entry and not a game
/// that never untaps anything, and a *named* colour out of a three-wide menu is
/// what tells "Add {G}, {U}, or {R}" from a rock that makes whatever it likes.
/// The mana line is read as the menu it is — a pool of one mana could not have
/// said which colour the land offered.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn ketria_triome_enters_tapped_cycles_for_a_card_and_taps_for_one_of_its_three_colours() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[ketria_triome(), ketria_triome()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // (1) The land drop. "This land enters tapped" is played rather than read,
    // and the reading that makes it worth anything is the offer it leaves
    // behind: a tapped land has no {T} to pay a mana line with, so neither list
    // names it until the untap step below stands it back up.
    let land = play_land(&mut engine, p0, ketria_triome());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" is a real entry and not a placement"
    );
    assert!(
        types(&engine, land).contains(TypeSet::LAND),
        "and what arrived is the land the card prints"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the seat holds priority after its land drop, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.mana_abilities.contains(&land)
            && !legal.abilities.iter().any(|(src, _)| *src == land),
        "a tapped land has no {{T}} left to pay a mana line with, so it is \
         offered nowhere: {:?} / {:?}",
        legal.mana_abilities,
        legal.abilities
    );

    // (2) Cycling {3} off the second copy, in hand. The cost is read off the
    // pool, so the Forests are tapped first — and the triome is named as the
    // printing kept back, since its own line is what step (4) reads.
    tap_all_mana_but(&mut engine, p0, Some(ketria_triome()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three tapped Forests, three green, and nothing off a triome"
    );
    let cycling = in_hand(&engine, p0, ketria_triome()).expect("the second copy is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let offered = deeds(&legal, &[cycling]);
    assert!(
        matches!(offered[..], [(0, Deed::Ability(_))]),
        "cycling is an activation out of the hand, so the offer names it: {offered:?}"
    );
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    engine
        .apply(p0, offered[0].1.action(cycling))
        .expect("the cycling line the offer named is affordable");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}} came out of the pool"
    );
    assert!(
        in_graveyard(&engine, p0, ketria_triome()).is_some(),
        "\"Discard this card\" is the other half of the price (CR 601.2h)"
    );
    assert!(
        in_hand(&engine, p0, ketria_triome()).is_none(),
        "and the copy that paid it is no longer in hand"
    );
    assert!(
        !stack_is_empty(&engine),
        "cycling is no mana ability, so the drawn card waits on the stack"
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

    // (3) A turn cycle, because nothing stands a tapped permanent back up but
    // its controller's own untap step (CR 502.3) — which is what separates the
    // printed entry from a land that simply never untapped.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood the triome back up, so it really was the entry \
         that tapped it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );

    // (4) The mana line. Three basic land types make the route ambiguous in the
    // engine — the CR 305.6 shortcut and the printed ability are two different
    // actions — so it is read off the offer rather than guessed at.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let by_shortcut = legal.mana_abilities.contains(&land);
    let by_printed = legal
        .abilities
        .iter()
        .find(|(source, _)| *source == land)
        .map(|(_, index)| *index);
    assert!(
        by_shortcut || by_printed.is_some(),
        "an untapped triome is a paid {{T}}, so its mana line is offered: {legal:?}"
    );
    let route = if by_shortcut {
        PlayerAction::ActivateManaAbility { source: land }
    } else {
        PlayerAction::ActivateAbility {
            source: land,
            ability_index: by_printed.expect("the assertion above enumerated both routes"),
        }
    };
    engine
        .apply(p0, route)
        .expect("the route the offer named is payable");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}}, {{U}}, or {{R}}\" is a choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped it names the colour");
    assert_eq!(
        options.len(),
        3,
        "the three colours its Forest, Island and Mountain types name, and no \
         fourth: {options:?}"
    );
    for color in [ManaColor::Green, ManaColor::Blue, ManaColor::Red] {
        assert!(
            options.contains(&color),
            "\"Add {{G}}, {{U}}, or {{R}}\" includes {color:?}: {options:?}"
        );
    }
    assert!(
        !options.contains(&ManaColor::Colorless),
        "colourless is no colour at all (CR 105.4): {options:?}"
    );
    assert!(
        !options.contains(&ManaColor::White) && !options.contains(&ManaColor::Black),
        "and neither colour the land does not name is on the menu: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the three it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, off one tap: nothing else on the board is still floating"
    );
    assert!(is_tapped(&engine, land), "the triome paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
}
