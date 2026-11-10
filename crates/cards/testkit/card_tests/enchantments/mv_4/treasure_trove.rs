//! `cards/enchantments/mv_4/treasure_trove.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Treasure Trove is `{2}{U}{U}` for one activated ability and no other text:
/// "`{2}{U}{U}`: Draw a card." The price is mana and **no tap symbol**, so one
/// board reads both halves of that. With an empty pool the line is not offered
/// at all — `legal.abilities` is filtered through `can_afford`, which reads the
/// pool rather than the eight untapped Islands — and eight Islands buy the
/// *same* enchantment two draws inside one turn, which a `{T}` in the cost
/// would forbid. Each draw is read on the library and the hand together, so an
/// emptied library could not stand in for it, and the Trove is asserted still
/// on the battlefield afterwards: the price is mana and no sacrifice.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn treasure_trove_buys_two_cards_off_one_enchantment_with_mana_and_no_tap() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                treasure_trove(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let trove = on_battlefield(&engine, p0, treasure_trove()).expect("the Trove is out");
    let kinds = types(&engine, trove);
    assert!(
        kinds.contains(TypeSet::ENCHANTMENT) && !kinds.contains(TypeSet::CREATURE),
        "it is the enchantment it prints: {kinds:?}"
    );

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        player, p0,
        "the seat with the Trove holds a quiet main phase"
    );
    assert!(
        !legal.abilities.contains(&(trove, 0)),
        "an empty pool pays no {{2}}{{U}}{{U}}, so nothing is offered: {:?}",
        legal.abilities
    );

    // Eight Islands: the first four pay for one activation and the second four
    // for another, because nothing in the price turns the source sideways.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "eight Islands, and the Trove makes no mana of its own"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        8,
        "and all eight make blue, so the {{U}}{{U}} in the cost has a source"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(trove, 0)),
        "with the mana floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, treasure_trove(), 0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the {{2}}{{U}}{{U}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the ability is on the stack"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it is in hand, so an emptied library would not satisfy the count above"
    );

    // The second draw is the half a `{T}` would forbid: the same enchantment is
    // offered again off the mana left over, and it was never tapped.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(trove, 0)),
        "no part of the price taps the Trove, so it is offered again: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, treasure_trove(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "both prices are paid and nothing is left of the eight"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 2,
        "two activations, two cards"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2,
        "and both reached the hand"
    );
    assert!(
        on_battlefield(&engine, p0, treasure_trove()).is_some(),
        "the price was mana and no sacrifice, so the Trove stays to draw again"
    );
}
