//! `cards/creatures/mv_3/fyndhorn_elder.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fyndhorn Elder is a 1/1 for {2}{G} whose whole printed text is one mana
/// ability: "{T}: Add {G}{G}." The scenario really casts it — three Forests pay
/// {2}{G} and the pool is empty when it lands — and then lets a full turn cycle
/// pass before pressing that ability, because a creature's own `{T}` is
/// unavailable the turn it arrives (CR 302.6) and a test that tapped it on
/// arrival would be measuring summoning sickness instead of the card.
///
/// Two green out of one activation is the claim, and nothing is tapped
/// beforehand so the empty pool makes it exact: `{G}{G}` names its colour and
/// asks nothing, which is why a `ChooseColor` here would mean the engine read a
/// printed {G} as a choice; and a mana ability resolves as it is activated
/// (CR 605.3b), so the mana and the tap are in place with an empty stack.
#[test]
fn fyndhorn_elder_taps_for_two_green_out_of_an_empty_pool() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[fyndhorn_elder()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {2}{G} off the three Forests, spent down to nothing by the cast.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests, three green"
    );
    cast_with_floating(&mut engine, p0, fyndhorn_elder());
    pass_until(&mut engine, stack_is_empty);
    let elder = on_battlefield(&engine, p0, fyndhorn_elder()).expect("the Elder resolved");
    assert_eq!(pt(&engine, elder), (1, 1), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{G}} took all three green with it, so the board floats nothing"
    );

    // Summoning sickness (CR 302.6): the Elder cannot pay its own {T} on the
    // turn it arrived, so the board waits out a turn cycle before the claim.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, elder),
        "and nothing tapped it on the way"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(elder, 0)),
        "a printed mana ability has an index to name, so the Elder's one line \
         is an ordinary entry in `abilities`: {:?}",
        legal.abilities
    );
    assert!(
        !legal.mana_abilities.contains(&elder),
        "the CR 305.6 shortcut is for mana a permanent has with no ability to \
         point at, which a printed {{T}} is not: {:?}",
        legal.mana_abilities
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the pool is empty, so whatever the next line adds is the Elder's alone"
    );

    activate(&mut engine, p0, fyndhorn_elder(), 0);

    assert!(
        !matches!(engine.pending(), Pending::ChooseColor { .. }),
        "the card names its colour, so there is nothing to choose: {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        2,
        "one activation, two green — a card printing a single {{G}} would leave one"
    );
    assert_eq!(
        pool.total(),
        2,
        "and nothing else: one tap, one colour, two mana"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, elder), "the Elder paid its own {{T}}");
}
