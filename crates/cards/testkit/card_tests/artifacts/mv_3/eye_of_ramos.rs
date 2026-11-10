//! `cards/artifacts/mv_3/eye_of_ramos.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Eye of Ramos is a `{3}` artifact printing the same blue twice at two
/// different prices: "{T}: Add {U}" and "Sacrifice this artifact: Add {U}".
/// One board plays both, because the card is the difference between them:
/// three Forests pay the `{3}` and leave the pool empty, so neither blue can
/// have come off a land; the first activation takes only the tap symbol and
/// leaves the artifact standing, and the second takes the artifact itself into
/// its owner's graveyard. The offer is read again in between, where a tapped
/// Eye has no `{T}` left to pay with and the sacrifice line still does.
#[test]
fn eye_of_ramos_taps_and_then_sacrifices_itself_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[eye_of_ramos()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {3} out of the three Forests, and nothing left floating: whatever blue
    // arrives below can only have come off the artifact itself.
    cast_from_hand(&mut engine, p0, eye_of_ramos());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let eye = on_battlefield(&engine, p0, eye_of_ramos()).expect("the Eye resolved");
    assert!(
        types(&engine, eye).contains(TypeSet::ARTIFACT),
        "what arrived is the artifact it prints"
    );
    assert!(!is_tapped(&engine, eye), "and it enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Forests paid the {{3}} and left nothing behind"
    );

    // Ability 0: "{T}: Add {U}". A fixed colour, so nothing is asked on the
    // way and the mana is in the pool the moment the tap resolves (CR 605.3b).
    activate(&mut engine, p0, eye_of_ramos(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1, "one blue off the tap");
    assert_eq!(pool.total(), 1, "and nothing else came with it");
    assert!(
        stack_is_empty(&engine),
        "a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, eye), "the Eye paid its own {{T}}");

    // The tap is spent, so the first line is no longer one the seat may take —
    // read with the mana already floating, because that is where the offer
    // reads its prices from.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(eye, 0)),
        "a tapped Eye has no {{T}} left to pay with: {:?}",
        legal.abilities
    );

    // Ability 1: "Sacrifice this artifact: Add {U}" — a price the tap symbol
    // cannot answer for, so it is still on the menu while the Eye is tapped.
    activate(&mut engine, p0, eye_of_ramos(), 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 2, "the second blue");
    assert_eq!(
        pool.total(),
        2,
        "two activations and two blue, with no land in either"
    );
    assert!(
        stack_is_empty(&engine),
        "the sacrifice line is a mana ability too (CR 605.3b)"
    );
    assert!(
        on_battlefield(&engine, p0, eye_of_ramos()).is_none(),
        "sacrificing the artifact is the price of the second line"
    );
    assert!(
        in_graveyard(&engine, p0, eye_of_ramos()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
}
