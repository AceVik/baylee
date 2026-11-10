//! `cards/creatures/mv_3/sisters_of_the_flame.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sisters of the Flame — {1}{R}{R} — a 2/2 Human Shaman whose entire printed
/// text is "{T}: Add {R}".
///
/// The card is played out of the hand for its three mana and arrives as the
/// body it prints, and the mana line is then read on a board where the one red
/// has nowhere else to come from: three Mountains are the only other
/// permanents, they are all still standing untapped, and the pool is empty on
/// both sides of the activation. The tap symbol is the whole price, which is
/// why the ability is offered with nothing floating, why it lands without a
/// stack (CR 605.3b) and why nothing is asked on the way — the card names its
/// colour, where a "one mana of any color" rock would have to ask.
#[test]
fn sisters_of_the_flame_is_a_two_two_that_taps_for_one_red() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[sisters_of_the_flame()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {1}{R}{R} off the three Mountains, and the pool is read after the cast,
    // so the three mana have really gone somewhere.
    cast_from_hand(&mut engine, p0, sisters_of_the_flame());
    pass_until(&mut engine, stack_is_empty);
    let sister = on_battlefield(&engine, p0, sisters_of_the_flame()).expect("the Sisters resolved");
    assert!(
        types(&engine, sister).contains(TypeSet::CREATURE),
        "a creature, and not some other permanent the board happened to hold"
    );
    assert_eq!(pt(&engine, sister), (2, 2), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Mountains paid {{1}}{{R}}{{R}} and nothing is left floating"
    );

    // The untap step is what puts the Mountains back, and that board is the
    // one the mana line is read on: nothing floating, three untapped Mountains
    // beside the Sister, and a creature that entered on the previous turn.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the Sisters' controller takes another turn"
    );
    let mountains = all_on_battlefield(&engine, p0, mountain());
    assert_eq!(mountains.len(), 3, "the three Mountains came back");
    assert!(
        mountains.iter().all(|id| !is_tapped(&engine, *id)),
        "every Mountain is standing untapped, so none of them paid for the \
         mana read below"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing floats, which is what makes `one red` an exact claim"
    );

    // The whole price is the tap symbol, so an untapped Sister is a paid
    // ability even on an empty pool: `can_afford` reads the permanent and not
    // the lands.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(sister, 0)),
        "the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, sisters_of_the_flame(), 0);
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "{{R}} is named and not chosen, so nothing was asked on the way: {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1, "{{T}}: Add {{R}}");
    assert_eq!(
        pool.total(),
        1,
        "one mana, off one tap, and nothing beside it"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the red is the Sister's: the only land types on this board make green"
    );
    assert!(is_tapped(&engine, sister), "the Sister paid her own {{T}}");
    assert!(
        mountains.iter().all(|id| !is_tapped(&engine, *id)),
        "and the Mountains never moved, so the red mana has no other source \
         on this board"
    );
}
