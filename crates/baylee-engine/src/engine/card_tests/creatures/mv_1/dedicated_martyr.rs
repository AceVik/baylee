//! `cards/creatures/mv_1/dedicated_martyr.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dedicated Martyr prints one line — "{W}, Sacrifice this creature: You gain
/// 3 life" — and both halves of that price are read off the board rather than
/// off the card file. The `{W}` is the mana pool: it is empty at the start, so
/// the ability is not even offered against an untapped Plains, and after the
/// activation the one white that was floated is gone, which is what says the
/// mana really left. The sacrifice is the zones: the Cleric is in its owner's
/// graveyard and off the battlefield. The opponent's own Martyr is the control
/// for "this creature" — it never moves, and its controller gains nothing.
#[test]
fn dedicated_martyr_sells_itself_for_three_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), dedicated_martyr()])
        .battlefield(1, &[dedicated_martyr()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let martyr = on_battlefield(&engine, p0, dedicated_martyr()).expect("the Cleric is out");
    let theirs = on_battlefield(&engine, p1, dedicated_martyr()).expect("their Cleric is out");

    // `legal.abilities` is filtered by what the pool can pay (CR 601.2h), not
    // by the untapped lands: with nothing floating there is no {W} to spend.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(martyr, 0)),
        "an empty pool affords no {{W}}, so the line is withheld: {:?}",
        legal.abilities
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "one Plains on the board is the whole pool, and the Cleric taps for nothing"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(martyr, 0)),
        "{{W}} floating is what offers the Cleric's only line: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == theirs),
        "and the offer is on my own Cleric, not the one across the table: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, dedicated_martyr(), 0);
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "nothing is asked on the way: the creature the cost sacrifices is the \
         one the ability is printed on"
    );

    assert_eq!(
        engine.state().players[0].life,
        23,
        "the resolving ability gains its controller three life"
    );
    assert_eq!(engine.state().players[1].life, 20, "and nobody else's");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{W}} left the pool, so the cost was a payment and not a label"
    );
    assert!(
        on_battlefield(&engine, p0, dedicated_martyr()).is_none(),
        "the Cleric sacrificed itself, so it is no longer on the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, dedicated_martyr()).is_some(),
        "a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, dedicated_martyr()).is_some(),
        "\"this creature\" is the one that activated: the opponent's Cleric \
         never moved"
    );
}
