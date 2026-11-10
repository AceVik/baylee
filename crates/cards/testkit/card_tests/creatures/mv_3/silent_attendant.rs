//! `cards/creatures/mv_3/silent_attendant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Silent Attendant is a {2}{W} 0/2 Human Cleric whose whole printed text is
/// "{T}: You gain 1 life." The board holds nothing else that can move a life
/// total — no mana, no spell, no permanent of the opponent's — so the single
/// life p0 gains can only have come off the tap the card prints. It is seated
/// rather than cast because the price is its own {T}: a creature that arrived
/// this turn could not pay it (CR 302.6) and the test would be asserting an
/// ability the engine never had reason to offer.
#[test]
fn silent_attendant_taps_for_one_life_and_leaves_the_other_seat_alone() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[silent_attendant()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let attendant = on_battlefield(&engine, p0, silent_attendant()).expect("the Attendant is out");
    assert_eq!(pt(&engine, attendant), (0, 2), "a printed 0/2 body");
    assert!(!is_tapped(&engine, attendant), "and it starts untapped");

    // The whole price is the tap symbol, so nothing has to be floating and no
    // source is tapped: the offer below is read off an empty pool.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the board holds no mana at all"
    );
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat with the Attendant holds it");
    assert!(
        legal.abilities.contains(&(attendant, 0)),
        "the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, silent_attendant(), 0);
    assert!(
        is_tapped(&engine, attendant),
        "{{T}} is the cost and is paid as the ability is activated"
    );
    assert!(
        !stack_is_empty(&engine),
        "gaining life is no mana ability, so the ability waits on the stack"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nothing has happened yet — the effect resolves off the stack"
    );

    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine.state().players[0].life,
        21,
        "\"You gain 1 life\" — one, on one activation"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life belongs to the seat that paid the price"
    );

    // The tap is spent, so the line is no longer one this seat may take.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(attendant, 0)),
        "a tapped Attendant has no {{T}} left to pay with: {:?}",
        legal.abilities
    );
}
