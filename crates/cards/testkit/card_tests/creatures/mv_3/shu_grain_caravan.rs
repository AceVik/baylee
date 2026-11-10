//! `cards/creatures/mv_3/shu_grain_caravan.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shu Grain Caravan — `{2}{W}` — Creature — Human Soldier 2/2: "When this
/// creature enters, you gain 2 life."
///
/// A life total read only at the end of the cast proves nothing on its own, so
/// the scenario reads both ends of it: while the Caravan is a *spell* on the
/// stack the controller is still at the twenty they started with (CR 603.6a
/// puts the trigger on entering and not on casting), and the three Plains are
/// empty of mana because `{2}{W}` is paid on announcement (CR 601.2h). The
/// opponent's life is the control for the pronoun — "you" is the controller,
/// not the table — and exactly two says the entry was counted once.
#[test]
fn shu_grain_caravan_gains_two_life_when_it_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[shu_grain_caravan()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, shu_grain_caravan());
    assert!(
        on_stack(&engine, shu_grain_caravan()).is_some(),
        "the Caravan is a creature *spell* until it resolves, so the trigger \
         that gains the life has not fired yet"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "nothing has entered, so nothing has been gained (CR 603.6a)"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the three Plains paid the {{2}}{{W}} when the spell was announced"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        22,
        "\"you gain 2 life\" — two, once, off one entry"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "\"you\" is the controller of the Caravan: the opponent's life never \
         moved"
    );
    let caravan = on_battlefield(&engine, p0, shu_grain_caravan()).expect("the Caravan resolved");
    assert_eq!(
        pt(&engine, caravan),
        (2, 2),
        "and the 2/2 body the card prints is standing on the battlefield"
    );
    assert!(
        all_on_battlefield(&engine, p0, plains())
            .iter()
            .all(|id| is_tapped(&engine, *id)),
        "the mana came out of the three Plains: every one of them is tapped"
    );
}
