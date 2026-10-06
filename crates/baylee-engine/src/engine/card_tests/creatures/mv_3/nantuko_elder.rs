//! `cards/creatures/mv_3/nantuko_elder.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nantuko Elder is a {2}{G} 1/2 Insect Druid whose whole printed text is one
/// mana ability: "{T}: Add {C}{G}." Two mana of two *different* kinds out of
/// one activation is the claim, and the board is built so that neither half
/// can be borrowed: the pool is empty when the Elder taps, the only other
/// permanents are Forests that make {G} and can never make {C}, and every one
/// of them is still standing afterwards — the colorless is the half no Forest
/// on this table could have produced at all.
///
/// A turn is spent before the tap because of CR 302.6: a creature's {T}
/// ability cannot be activated the turn it arrives, so the Elder is cast,
/// walked around to its controller's next main phase, and tapped there with
/// the Forests untapped and the pool still empty.
#[test]
fn nantuko_elder_taps_for_one_colorless_and_one_green_the_turn_after_it_arrives() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[nantuko_elder()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {2}{G} off the three Forests, every one of which is spent: a 1/2 body
    // that arrives with an empty pool is what makes the two mana below the
    // Elder's own rather than a land's.
    cast_from_hand(&mut engine, p0, nantuko_elder());
    pass_until(&mut engine, stack_is_empty);
    let elder = on_battlefield(&engine, p0, nantuko_elder()).expect("the Elder resolved");
    assert_eq!(pt(&engine, elder), (1, 2), "the printed 1/2 body");
    assert!(
        types(&engine, elder).contains(TypeSet::CREATURE),
        "an Insect Druid is a creature and nothing else: {:?}",
        types(&engine, elder)
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Forests paid the {{2}}{{G}} exactly, so nothing is floating"
    );
    assert!(!is_tapped(&engine, elder), "and the Elder is untapped");

    // Around to its controller's next turn: the untap step stands the Forests
    // back up, and CR 302.6 has stopped applying to the Elder.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(forests.len(), 3, "the same three Forests are still there");
    assert!(
        forests.iter().all(|id| !is_tapped(&engine, *id)),
        "the untap step ran, so a green source stands beside the Elder"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing is floating"
    );

    // Ability 0 is the printed "{T}: Add {C}{G}", and it is an ordinary
    // `abilities` entry rather than the CR 305.6 shortcut: a card that prints
    // its own mana line has an index to name.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(elder, 0)),
        "the whole price is the tap symbol, so the line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, nantuko_elder(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "{{C}} — which no Forest on this board could have made"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "and {{G}} beside it, off the same activation"
    );
    assert_eq!(pool.total(), 2, "two mana, one tap, and nothing else");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, elder), "the Elder paid its own {{T}}");
    assert!(
        forests.iter().all(|id| !is_tapped(&engine, *id)),
        "and every Forest is still standing, so the colorless came from the \
         Elder and not from a land"
    );
}
