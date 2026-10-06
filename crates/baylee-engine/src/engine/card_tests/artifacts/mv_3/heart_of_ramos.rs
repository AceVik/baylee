//! `cards/artifacts/mv_3/heart_of_ramos.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Heart of Ramos prints two mana abilities and no other text: "{T}: Add {R}"
/// and "Sacrifice this artifact: Add {R}". The board is three Forests and
/// nothing else, so they pay the printed {3} down to an empty pool and the red
/// that arrives afterwards has no other source it could have come from. Each
/// price is read where it lands — the tap as a status change on a permanent
/// that stays, the sacrifice as a graveyard entry on one that does not — and
/// the empty stack after each says what kind of ability this is (CR 605.3b).
#[test]
fn heart_of_ramos_taps_and_then_sacrifices_itself_for_red_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[heart_of_ramos()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Three Forests pay exactly the printed {3}, so the pool the two mana
    // abilities write into starts blank: "one red" below is then a claim about
    // the artifact and not about a green left floating beside it.
    cast_from_hand(&mut engine, p0, heart_of_ramos());
    pass_until(&mut engine, stack_is_empty);
    let heart = on_battlefield(&engine, p0, heart_of_ramos()).expect("the Heart resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{3}} out of exactly three Forests leaves nothing floating"
    );

    // The printed mana ability is an ordinary `(source, index)` entry in
    // `LegalActions::abilities` and not the CR 305.6 shortcut: it has an index
    // to name, and its whole price is its own tap.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(heart, 0)),
        "an untapped artifact is a paid {{T}}: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, heart_of_ramos(), 0);
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "`{{T}}: Add {{R}}` — and the Forests beside it make green"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, heart), "the Heart paid its own {{T}}");

    // The second printed line. Its price is the artifact itself and not its
    // tap, so it is still offered to a Heart that is already tapped.
    activate(&mut engine, p0, heart_of_ramos(), 1);
    assert!(
        stack_is_empty(&engine),
        "the sacrifice is a mana ability too: no stack, and the mana is here"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "one red from the tap and one from the sacrifice"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and nothing else in the pool"
    );
    assert!(
        on_battlefield(&engine, p0, heart_of_ramos()).is_none(),
        "sacrificing the Heart is half of that price, so it left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, heart_of_ramos()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
}
