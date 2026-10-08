//! `cards/enchantments/mv_4/smothering_tithe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn smothering_tithe_card() -> CardIndex {
    card_index("153376c9-dffd-458c-8ce3-a4c8269bc4e9")
}

/// A table of four with the Tithe on seat 0, walked to the first draw-step
/// question: seat 1's turn, so seat 1 is asked.
fn a_tithe_asking_seat_one() -> Engine<RegistryLookup> {
    let mut engine = Duel::table(SEED, island(), 4)
        .battlefield(0, &[smothering_tithe_card()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    engine
}

/// "Whenever an opponent draws a card, that player may pay {2}": the
/// question goes to the opponent who drew, for {2}, and to nobody else.
#[test]
fn smothering_tithe_asks_only_the_drawing_opponent_for_two() {
    let engine = a_tithe_asking_seat_one();
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending()
    else {
        unreachable!("the helper stops on the tax question")
    };
    assert_eq!(*mana, 2, "the Tithe prints {{2}}");
    assert_eq!(
        *player,
        PlayerId::new(1),
        "seat 1 drew in its own draw step"
    );
    assert_eq!(
        engine.awaited().iter().collect::<Vec<_>>(),
        vec![PlayerId::new(1)],
        "no other seat has a question"
    );
    for other in [0, 2, 3] {
        assert!(
            engine.pending_for(PlayerId::new(other)).is_none(),
            "seat {other} is asked nothing"
        );
    }
}

/// Declined: the Tithe's controller, not the drawing opponent, gets the
/// Treasure, and only one for the one draw.
#[test]
fn smothering_tithe_declined_gives_its_controller_a_treasure() {
    let mut engine = a_tithe_asking_seat_one();
    let p0 = PlayerId::new(0);
    assert!(treasures(&engine, p0).is_empty(), "no Treasure yet");

    engine
        .apply(PlayerId::new(1), PlayerAction::YesNo(false))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(treasures(&engine, p0).len(), 1, "the tax went unpaid");
    for other in [1, 2, 3] {
        assert!(
            treasures(&engine, PlayerId::new(other)).is_empty(),
            "seat {other} gets no Treasure"
        );
    }
}

/// Paid: no Treasure. The drawing opponent has mana floating when it is
/// asked (put there by the harness' own dev capability, since nothing makes
/// mana in a draw step), and the payment takes exactly {2} of it.
#[test]
fn smothering_tithe_paid_gives_nobody_a_treasure() {
    let mut engine = a_tithe_asking_seat_one();
    let p1 = PlayerId::new(1);
    engine
        .dev_state_mut(p1)
        .expect("the harness may set boards up")
        .players[1]
        .mana_pool
        .add(ManaColor::Green, 3);
    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        1,
        "the tax took {{2}} of the three"
    );
    for seat in 0..4 {
        assert!(
            treasures(&engine, PlayerId::new(seat)).is_empty(),
            "seat {seat}: paid, so no Treasure"
        );
    }
}
