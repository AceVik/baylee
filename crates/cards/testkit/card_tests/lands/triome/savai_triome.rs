//! `cards/lands/triome/savai_triome.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "00625242-9348-4ef4-b975-f2ac82fee21d"

/// Savai Triome is a triome: a land with three basic land types printing "This
/// land enters tapped", "{T}: Add {R}, {W}, or {B}", and cycling "{3}, Discard
/// this card: Draw a card" out of the hand.
///
/// The entry modifier is the half a `starting_battlefield` placement never runs
/// (`Cause::Setup` looks at no replacement effect), so the land is played as a
/// real land drop and everything after it is read on the board that entry
/// produced: nothing offered on the arrival turn, back up in its controller's
/// untap step, and only then the one question — three colours wide, with blue,
/// green and colourless absent. Cycling is the second printed line and its
/// `DiscardSelf` is read in the graveyard, which is what makes the net-zero
/// hand count a claim about a cost *and* a draw rather than about either alone.
#[test]
#[allow(clippy::too_many_lines)] // two printed lines, and the second needs the first turn to have passed
fn savai_triome_enters_tapped_taps_for_one_of_its_three_colors_and_cycles_for_three() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[savai_triome(), savai_triome()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, savai_triome());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — a real land drop, not a placement"
    );
    assert!(
        types(&engine, land).contains(TypeSet::LAND),
        "what arrived is the land it prints"
    );
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "p0 keeps priority after playing a land, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "and it is the seat that played it");
    assert!(
        !legal.abilities.contains(&(land, 0)),
        "an untapped land is the whole price of its own mana ability, so a land \
         that entered tapped has nothing to offer: {:?}",
        legal.abilities
    );

    // A whole turn cycle: the untap step is what turns the printed {T} into an
    // ability the seat is offered at all, and nothing else on this board can.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood the land back up"
    );

    // Everything but the Triome, whose {T} is the tap this test presses by
    // hand: four Forests are four green, and the Triome contributes none of it.
    tap_all_mana_but(&mut engine, p0, Some(savai_triome()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests tapped, and the Triome was the one thing kept back"
    );

    // Ability 0 is the printed "{T}: Add {R}, {W}, or {B}".
    activate(&mut engine, p0, savai_triome(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{R}}, {{W}}, or {{B}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        3,
        "the three colours the card prints and no fourth: {options:?}"
    );
    for color in [ManaColor::Red, ManaColor::White, ManaColor::Black] {
        assert!(
            options.contains(&color),
            "\"{{R}}, {{W}}, or {{B}}\" includes {color:?}: {options:?}"
        );
    }
    assert!(
        !options.contains(&ManaColor::Colorless),
        "colourless is no colour at all (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        4,
        "and one mana, not a second helping of the Forests' green"
    );
    assert_eq!(pool.total(), 5, "the four Forests and the one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert!(is_tapped(&engine, land), "the Triome paid its own {{T}}");

    // The second printed line: cycling, which is activated from the *hand* and
    // costs {3} plus the card itself. The copy on the battlefield is the
    // witness that the zone is read — a tapped land sitting there offers it.
    let copy = in_hand(&engine, p0, savai_triome()).expect("the second copy is in hand");
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == copy)
        .expect("cycling is offered from the hand once its three mana is payable");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the mana already floating pays for the cycling");

    assert!(
        in_graveyard(&engine, p0, savai_triome()).is_some(),
        "`Discard this card` is paid with the ability, so the card is in its \
         owner's graveyard before anything resolves"
    );
    assert!(
        !stack_is_empty(&engine),
        "cycling is no mana ability, so the ability is waiting on the stack"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{3}} came out of the pool the Forests and the Triome filled"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "one card off the top of the library, which is the whole of the payoff"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card discarded and one drawn: the hand is the size it was, so \
         neither half happened without the other"
    );
    assert!(
        on_battlefield(&engine, p0, savai_triome()).is_some(),
        "and the copy on the battlefield was never the one cycled"
    );
}
