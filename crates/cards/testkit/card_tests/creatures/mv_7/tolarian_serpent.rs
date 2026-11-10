//! `cards/creatures/mv_7/tolarian_serpent.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tolarian Serpent is a {5}{U}{U} 7/7 whose entire text is "At the beginning
/// of your upkeep, mill seven cards."
///
/// The possessive is the word worth a witness, and the table supplies one for
/// free: the Serpent is cast in its controller's own first main phase, so the
/// walk has to cross the *opponent's* whole turn before the controller's next
/// upkeep arrives — an implementation that fired on any upkeep would empty a
/// library before the assertion below is ever reached. Both graveyards are
/// read beside both libraries, because "mill seven" is a move between zones
/// and a library that merely shrank would not say where the seven went.
#[test]
fn tolarian_serpent_mills_seven_on_its_controllers_upkeep_and_never_on_the_opponents() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 7])
        .hand(0, &[tolarian_serpent()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Seven Islands are exactly {5}{U}{U}, so the Serpent arrives off the whole
    // board and nothing is left floating behind it.
    cast_from_hand(&mut engine, p0, tolarian_serpent());
    pass_until(&mut engine, stack_is_empty);
    let serpent = on_battlefield(&engine, p0, tolarian_serpent()).expect("the Serpent resolved");
    assert_eq!(pt(&engine, serpent), (7, 7), "the body the card prints");

    let hand_size = |engine: &Engine<RegistryLookup>, seat: PlayerId| {
        engine.state().zones.list(ZoneLocation::Hand(seat)).len()
    };
    let my_library = library_size(&engine, p0);
    let their_library = library_size(&engine, p1);
    let my_hand = hand_size(&engine, p0);
    let their_hand = hand_size(&engine, p1);
    let my_yard = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    let their_yard = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();
    assert!(my_library > 7, "seven cards to mill, and more besides");

    // The opponent's turn comes first, and the printed sentence says nothing
    // about it: a Serpent that milled on any upkeep would empty a library here.
    // The opponent does lose one card off the top — their own draw step
    // (CR 504.1) — so their library is read beside their hand, where a draw
    // lands and a mill does not.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        library_size(&engine, p0),
        my_library,
        "\"at the beginning of *your* upkeep\" — the controller's library is \
         untouched on a turn that is not theirs"
    );
    assert_eq!(
        hand_size(&engine, p1),
        their_hand + 1,
        "the opponent drew for their turn"
    );
    assert_eq!(
        library_size(&engine, p1),
        their_library - 1,
        "and that draw is the only card their library lost: nothing mills the \
         opponent either"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        their_yard,
        "their graveyard is where a mill would have put the cards, and it is empty"
    );

    // Back around to the controller's own turn, whose upkeep the sentence is
    // about. The draw step follows the upkeep, so the controller's library pays
    // for one draw beside the seven milled cards, and the hand says which one.
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        hand_size(&engine, p0),
        my_hand + 1,
        "the controller drew for their turn, after the upkeep"
    );
    assert_eq!(
        library_size(&engine, p0),
        my_library - 7 - 1,
        "\"mill seven cards\": seven off the top of the controller's library, \
         and the one card the draw step took"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        my_yard + 7,
        "and the seven are in the graveyard, not merely gone from the library"
    );
    assert_eq!(
        library_size(&engine, p1),
        their_library - 1,
        "the opponent's library is still only its own draw short after the \
         second upkeep"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        their_yard,
        "and so is their graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, tolarian_serpent()).is_some(),
        "milling costs the Serpent nothing: it is still the permanent it entered as"
    );
}
