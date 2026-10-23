//! `cards/artifacts/mv_2/anvil_of_bogardan.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn anvil_of_bogardan() -> CardIndex {
    card_index("9deb2a0b-f40c-4d13-8321-41dc41448d24")
}

/// Answers the loot question the Anvil asks at `seat`'s draw step: the seat
/// whose draw step it is chooses, from a hand of two (the card drawn and the
/// extra one), and the stack empties.
#[track_caller]
fn discard_first_option(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    pass_until(engine, |e| {
        e.state().turn.active == seat && matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, seat, "the player whose draw step it is discards");
    assert_eq!(options.len(), 2, "one drawn card and one extra card");
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
}

/// Anvil of Bogardan (`{2}`, `Coverage::Partial`): "At the beginning of each
/// player's draw step, that player draws an additional card, then discards a
/// card." The opponent's draw step loots the opponent (two cards drawn, one
/// discarded, one kept) and not the Anvil's controller; the controller's own
/// draw step loots the controller.
#[test]
fn anvil_of_bogardan_loots_each_player_in_their_own_draw_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[anvil_of_bogardan()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, anvil_of_bogardan());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, anvil_of_bogardan()).is_some());

    // p1's draw step.
    discard_first_option(&mut engine, p1);
    let graveyard = |engine: &Engine<RegistryLookup>, seat| {
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(seat))
            .len()
    };
    let hand = |engine: &Engine<RegistryLookup>, seat| {
        engine.state().zones.list(ZoneLocation::Hand(seat)).len()
    };
    assert_eq!(graveyard(&engine, p1), 1, "one card discarded");
    assert_eq!(hand(&engine, p1), 1, "two drawn, one discarded");
    assert_eq!(graveyard(&engine, p0), 0, "p1's draw step does not loot p0");

    // p0's draw step: the Anvil's controller is looted too ("each player").
    discard_first_option(&mut engine, p0);
    assert_eq!(graveyard(&engine, p0), 1, "one card discarded");
    assert_eq!(hand(&engine, p0), 1, "two drawn, one discarded");
    assert_eq!(graveyard(&engine, p1), 1, "p0's draw step does not loot p1");
}

/// "Players have no maximum hand size", for the Anvil's controller (the
/// opponent's half is the gap): ten cards in hand at the end of the turn
/// are all kept, where seven is the limit without the Anvil.
#[test]
fn anvil_of_bogardan_keeps_its_controllers_oversized_hand_at_cleanup() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut hand = vec![anvil_of_bogardan()];
    hand.extend([forest(); 10]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, anvil_of_bogardan());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        10,
        "ten forests left in hand"
    );

    // Through p0's cleanup step (a discard question would stop pass_until).
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        10,
        "no discard down to seven"
    );
}
