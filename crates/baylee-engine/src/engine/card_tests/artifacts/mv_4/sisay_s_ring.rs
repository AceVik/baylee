//! `cards/artifacts/mv_4/sisay_s_ring.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sisay's Ring prints one line — "{T}: Add {C}{C}" — and both halves of it
/// are the engine's answer rather than the card's. Four Forests pay the `{4}`
/// down to an exactly empty pool, so the two colourless that arrive afterwards
/// have no other source on the board, and the whole price of the ability is
/// the Ring's own tap, so it is offered before a single mana is floating.
/// `{C}` is fixed rather than chosen, so nothing is asked on the way and the
/// mana lands with an empty stack (CR 605.3b); the second look at the offer,
/// with the Ring down, is what tells that tap from a label on a free ability.
#[test]
fn sisays_ring_taps_for_two_colorless_and_offers_nothing_once_it_is_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[sisay_s_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four Forests are the whole cost, so the pool reads empty the moment the
    // artifact lands and nothing floating can be mistaken for what follows.
    cast_from_hand(&mut engine, p0, sisay_s_ring());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let ring = on_battlefield(&engine, p0, sisay_s_ring()).expect("the Ring resolved");
    assert!(
        types(&engine, ring).contains(TypeSet::ARTIFACT),
        "what arrived is the artifact it prints"
    );
    assert!(!is_tapped(&engine, ring), "and an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "four Forests paid the {{4}} to the last mana"
    );

    // The whole price is the tap symbol, so the line is payable on an empty
    // pool — which is what makes the offer itself a reading.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(ring, 0)),
        "an untapped Ring is a paid {{T}}, so the one line the card prints is \
         offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, sisay_s_ring(), 0);
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "`{{C}}{{C}}` is fixed, so there is nothing to name on the way \
         (CR 605.1), got {:?}",
        engine.pending()
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        2,
        "\"{{T}}: Add {{C}}{{C}}\" — two, off one tap"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the four Forests are spent and make green besides, so nothing still \
         standing on this board could have produced the two"
    );
    assert_eq!(pool.total(), 2, "two mana, and nothing else came with them");
    assert!(is_tapped(&engine, ring), "the Ring paid its own {{T}}");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(ring, 0)),
        "a tapped Ring has no {{T}} left to pay with: {:?}",
        legal.abilities
    );
}
