//! `cards/lands/cycling/blasted_landscape.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blasted Landscape prints two lines, and one of them is the whole card while
/// it sits in a hand: "{T}: Add {C}" on the battlefield, and "Cycling {2}"
/// behind `ActivationZone::Hand`. Two copies are dealt so both lines are played
/// in one main phase — the first arrives as a land drop and is tapped, the
/// second is cycled out of the hand.
///
/// The colourless half is read off a pool nothing else could have filled: the
/// only permanent untapped when the tap is pressed is the land itself, so the
/// Forests beside it prove the mana came from the card and not from the board.
/// The cycling half is asked for twice, once at one mana floating and once at
/// three, because "{2}" is a price the offer is filtered on (CR 601.2h) rather
/// than a label — and the pool is zero afterwards, which two paid-for green
/// does not leave behind.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn blasted_landscape_taps_for_colorless_and_cycles_itself_out_of_the_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[blasted_landscape(), blasted_landscape()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The first copy arrives the only way this card ever arrives: a land drop.
    let played = play_land(&mut engine, p0, blasted_landscape());
    assert!(
        !is_tapped(&engine, played),
        "no enters-tapped clause is printed, so the land is ready the turn it arrives"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a land drop hands priority straight back, got {:?}",
            engine.pending()
        )
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == played)
        .expect("the printed {T}: Add {C} is an ordinary ability with an index");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "{{T}}: Add {{C}} — one colourless mana, in the pool the moment it is \
         activated (CR 605.3b)"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana off one tap: the Forests are still standing, so nothing else \
         on this board could have made it"
    );
    assert!(stack_is_empty(&engine), "a mana ability uses no stack");

    // The second copy, still in hand, which is where cycling lives.
    let cycling = in_hand(&engine, p0, blasted_landscape()).expect("the second copy is in hand");
    let library_before = library_size(&engine, p0);
    let offered = |engine: &Engine<RegistryLookup>| -> bool {
        matches!(
            engine.pending(),
            Pending::Priority { legal, .. }
                if legal.abilities.iter().any(|(src, _)| *src == cycling)
        )
    };
    assert!(
        !offered(&engine),
        "one colourless is not {{2}}: `legal.abilities` is filtered on what the \
         pool can pay, so the ability is not on the list yet"
    );

    // {2} is generic, and the two Forests are the only untapped things left.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the two Forests on top of the colourless the land already made"
    );
    assert!(offered(&engine), "and at three the price is met");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("tapping for mana leaves the seat holding priority")
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == cycling)
        .expect("the ability the offer just named");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, blasted_landscape()).is_some(),
        "DiscardSelf names the card the ability is printed on, so the copy that \
         cycled is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1,
        "and only that copy: the permanent in play is not part of the cost"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\" off the cycling ability"
    );
    assert!(
        in_hand(&engine, p0, blasted_landscape()).is_none(),
        "the copy that cycled left the hand"
    );
    assert!(
        on_battlefield(&engine, p0, blasted_landscape()).is_some(),
        "while the copy that was played is still a land on the battlefield"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "{{2}} was actually spent — a discard that paid nothing would have left \
         the two green floating"
    );
}
