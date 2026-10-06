//! `cards/creatures/mv_2/wretched_anurid.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wretched Anurid is a {1}{B} 3/3 whose whole text is "Whenever another
/// creature enters, you lose 1 life." Two words carry the card, and each needs
/// a different arrival to be read: *another*, because the Anurid entering must
/// not drain the seat that played it, and the missing "you control", because
/// the drain is the Anurid's controller's life whoever's creature entered.
/// Three resolutions are played in one game — the Anurid, a second creature
/// under the same seat, and one across the table — and the life total of both
/// seats is read after each, so a trigger that fired on its own arrival, or
/// only on its controller's creatures, or on the wrong seat, fails one of them.
#[test]
fn wretched_anurid_drains_its_controller_for_another_creature_wherever_it_comes_from() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), forest()])
        .hand(0, &[wretched_anurid(), llanowar_elves()])
        .battlefield(1, &[forest()])
        .hand(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Two Swamps pay {1}{B} and the Forest is the thing kept back: it is the
    // source the second creature is cast off, and a Forest spent here would
    // make the second cast a claim about a board that had none.
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    tap_mana_except(&mut engine, p0, land);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Swamps, and nothing off the Forest"
    );
    cast_with_floating(&mut engine, p0, wretched_anurid());
    pass_until(&mut engine, |e| stack_is_empty(e) && at_rest(e, p0));

    let anurid = on_battlefield(&engine, p0, wretched_anurid()).expect("the Anurid resolved");
    assert_eq!(pt(&engine, anurid), (3, 3), "the printed 3/3 body");
    assert_eq!(
        engine.state().players[0].life,
        20,
        "\"another creature\": the Anurid's own arrival is not one, so nothing \
         is lost for it"
    );

    // A second creature under the same seat, so the drain can only come from
    // the printed trigger and not from a cost or a state-based effect.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the Forest that was held back, and nothing else on the board"
    );
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, |e| stack_is_empty(e) && at_rest(e, p0));
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"whenever another creature enters, you lose 1 life\" — one creature, \
         one life"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life is the Anurid's controller's, not the table's"
    );

    // The half a narrower reading would lose. The card prints no "you
    // control", so the opponent's own Elf entering drains p0 again while p1
    // pays nothing for it.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        1,
        "the opponent's one Forest, one green"
    );
    cast_with_floating(&mut engine, p1, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature across the table resolved"
    );
    assert_eq!(
        engine.state().players[0].life,
        18,
        "a creature entering under an opponent is still *another* creature"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the Elf's own controller pays none of that life"
    );
}
