//! `cards/lands/utility/rustic_clachan.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rustic Clachan prints `As this land enters, you may reveal a Kithkin card from your hand. If you don't, this land enters tapped.`, `{{T}}: Add {{W}}.`, and `Reinforce 1—{{1}}{{W}} ({{1}}{{W}}, Discard this card: Put a +1/+1 counter on target creature.)`
///
/// Under `Coverage::Partial`, the enters-tapped-unless-reveal-Kithkin clause is omitted because entry modifiers cannot inspect cards in a player's hand.
/// Playing Rustic Clachan enters the battlefield untapped.
/// With a creature on the battlefield and `{{1}}{{W}}` floating from the played land and a basic land, a second copy of Rustic Clachan in hand offers its Reinforce 1 ability.
/// Activating ability 1 discards the card from hand, places a +1/+1 counter on the creature, and leaves the discarded card in the graveyard.
#[test]
fn rustic_clachan_enters_untapped_and_reinforces_from_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), young_wolf()])
        .hand(0, &[rustic_clachan(), rustic_clachan(), crib_swap()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let clachan = play_land(&mut engine, p0, rustic_clachan());
    // "You may reveal a Kithkin card from your hand." This pool prints no
    // Kithkin at all, so the only card that answers is a changeling —
    // CR 702.73a makes Crib Swap every creature type in every zone.
    let kithkin = in_hand(&engine, p0, crib_swap()).expect("Crib Swap is in hand");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![kithkin],
            },
        )
        .expect("a changeling is a Kithkin card");
    assert!(
        !entered_tapped(&engine, clachan),
        "revealing a Kithkin card keeps the Clachan untapped"
    );

    // Float {{1}}{{W}} from Forest and Rustic Clachan.
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.iter().any(|(source, idx)| *idx == 1
            && engine
                .state()
                .object(*source)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == rustic_clachan()))),
        "the second rustic clachan in hand offers its reinforce ability"
    );

    activate(&mut engine, p0, rustic_clachan(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("wolf on battlefield");
    assert!(options.contains(&wolf));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wolf],
            },
        )
        .unwrap();

    assert!(
        in_graveyard(&engine, p0, rustic_clachan()).is_some(),
        "the reinforced card is discarded to the graveyard as cost"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(counters_on(&engine, wolf, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, wolf), (2, 2));
}

/// Rustic Clachan prints "As this land enters, you may reveal a Kithkin card from your hand. If you don't, this land enters tapped.", `{{T}}: Add {{W}}.`, and `Reinforce 1—{{1}}{{W}}`.
///
/// Under `Coverage::Implemented`, the land checks for a Kithkin card in hand upon entry.
/// Revealing a card with changeling (`Crib Swap`) satisfies the creature type check and allows the land to enter untapped while keeping the card in hand, after which activating its mana ability produces White mana; without a Kithkin card in hand, it enters tapped.
#[test]
fn rustic_clachan_enters_untapped_by_revealing_kithkin() {
    let p0 = PlayerId::new(0);
    let changeling = card_index("2987c385-011a-4032-a516-a46d1e9dc9e8");
    let mut engine = Duel::new(160, forest())
        .hand(0, &[rustic_clachan(), changeling])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, rustic_clachan());
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected reveal prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (0, 1));
    assert_eq!(prompt, ChoicePrompt::RevealOrEnterTapped);
    let shown = in_hand(&engine, p0, changeling).expect("changeling is in hand");
    assert_eq!(options, vec![shown]);

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![shown],
            },
        )
        .expect("reveal kithkin card");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, land),
        "revealing a Kithkin lets Rustic Clachan enter untapped"
    );
    assert!(
        in_hand(&engine, p0, changeling).is_some(),
        "revealed card remains in hand"
    );

    activate(&mut engine, p0, rustic_clachan(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));

    let mut engine2 = Duel::new(161, forest())
        .hand(0, &[rustic_clachan()])
        .start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, p0);

    let land2 = play_land(&mut engine2, p0, rustic_clachan());
    assert!(
        is_tapped(&engine2, land2),
        "with no Kithkin in hand, Rustic Clachan enters tapped"
    );
}
