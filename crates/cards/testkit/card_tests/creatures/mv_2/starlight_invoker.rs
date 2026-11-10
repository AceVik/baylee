//! `cards/creatures/mv_2/starlight_invoker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Starlight Invoker — {1}{W}, a 1/3 Human Cleric Mutant — prints exactly one
/// line: "{7}{W}: You gain 5 life."
///
/// Ten lands go into the pool before anything is claimed, because
/// `can_afford` reads the mana pool and not the untapped lands: the cast's
/// {1}{W} comes out of that pool and leaves exactly the eight the ability
/// charges, which is the only moment the {7}{W} is legible at all — on an
/// empty pool the activation would be missing for want of mana rather than for
/// anything about the card. The five life is read on the seat that paid it,
/// and the pool is empty afterwards, so the life cannot have been bought with
/// mana that was never spent. Both left over — the {1}{W} of the cast and the
/// {7}{W} of the ability — are real payments out of a pool ten lands filled.
#[test]
fn starlight_invoker_charges_eight_mana_for_five_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[starlight_invoker()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 10, "three Plains and seven Forests");
    assert_eq!(pool.available(ManaColor::White), 3, "the Plains");
    assert_eq!(pool.available(ManaColor::Green), 7, "the Forests");

    cast_with_floating(&mut engine, p0, starlight_invoker());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let invoker = on_battlefield(&engine, p0, starlight_invoker()).expect("the Invoker resolved");
    assert_eq!(pt(&engine, invoker), (1, 3), "the body it prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "{{1}}{{W}} out of the pool leaves exactly the eight the ability charges"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(invoker, 0)),
        "{{7}}{{W}} is eight, and the pool holds exactly eight: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, starlight_invoker(), 0);
    assert!(!stack_is_empty(&engine), "gaining life is no mana ability");
    assert_eq!(
        engine.state().players[0].life,
        20,
        "nothing has resolved yet — the effect arrives off the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        25,
        "\"You gain 5 life\" — one activation, five life"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life belongs to the seat that paid, not to the opponent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{7}}{{W}} came out of the pool"
    );
    assert!(
        on_battlefield(&engine, p0, starlight_invoker()).is_some(),
        "the ability costs its controller nothing but the mana — the Invoker stays"
    );
}
