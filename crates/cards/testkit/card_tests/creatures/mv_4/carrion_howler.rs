//! `cards/creatures/mv_4/carrion_howler.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Carrion Howler is `{3}{B}` for a printed 2/2 whose whole rules text is
/// "Pay 1 life: This creature gets +2/−1 until end of turn." The price names
/// no mana and no tap, so the one line has to be offered over an *empty*
/// pool and has to leave the creature standing, which is exactly what a board
/// of tapped Swamps can tell apart from a `{T}` or a mana cost. And because
/// nothing is tapped it may be paid twice, which buys the half of the printed
/// numbers a single read cannot: `+2/−1` twice is a 6/0 that CR 704.5f buries,
/// where a `+2/+0` would have left a live 6/2 behind.
#[test]
fn carrion_howler_pays_life_to_grow_and_can_spend_itself_to_death() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[carrion_howler()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, carrion_howler());
    pass_until(&mut engine, stack_is_empty);
    let howler = on_battlefield(&engine, p0, carrion_howler()).expect("the Howler resolved");
    assert_eq!(pt(&engine, howler), (2, 2), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "four Swamps pay {{3}}{{B}} exactly, so nothing is left floating"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and no life is gone before anything is activated"
    );

    // The price is one life, so the offer does not read the pool at all: the
    // line is there with nothing floating, which is the half a `{T}` or a
    // mana cost would not have.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(howler, 0)),
        "the whole price is one life and no mana, so the line is offered on \
         an empty pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, carrion_howler(), 0);
    assert_eq!(
        engine.state().players[0].life,
        19,
        "CR 601.2h: the life is paid as the ability is announced"
    );
    assert!(
        !stack_is_empty(&engine),
        "paying life is no mana ability, so the pump is on the stack"
    );
    assert_eq!(
        pt(&engine, howler),
        (2, 2),
        "and nothing is pumped yet — the effect has not resolved"
    );
    assert!(
        !is_tapped(&engine, howler),
        "the price is life, not the creature's own {{T}}"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, howler), (4, 1), "+2/-1, both halves of it");
    assert!(
        !is_tapped(&engine, howler),
        "so the same creature may pay again: the printed cost names no tap"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and no mana went anywhere near the activation"
    );

    // A 2/2 that has paid twice is a 6/0, and state-based actions are the only
    // thing that can say so: a `+2/+0` would leave a live 6/2 standing here.
    activate(&mut engine, p0, carrion_howler(), 0);
    pass_until(&mut engine, |e| {
        in_graveyard(e, p0, carrion_howler()).is_some()
    });
    assert_eq!(
        engine.state().players[0].life,
        18,
        "one life per activation, and the second one is gone too"
    );
    assert!(
        on_battlefield(&engine, p0, carrion_howler()).is_none(),
        "two -1s on a printed 2/2 is zero toughness"
    );
}
