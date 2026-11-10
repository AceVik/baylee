//! `cards/enchantments/mv_3/moonlit_wake.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// CR 603.10a: a dies trigger looks back at the object as it last existed on
/// the battlefield. A Forest that Living Lands made a 1/1 creature dies as a
/// creature, so Moonlit Wake's "whenever a creature dies" pays for it, even
/// though the card in the graveyard is a Forest and no creature. The Plains
/// beside it was never a creature and pays nothing. Read off the card in the
/// graveyard, the Forest paid nothing either.
#[test]
fn a_land_that_died_as_a_creature_is_a_creature_dying() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[moonlit_wake(), living_lands(), forest(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let forest_id = on_battlefield(&engine, p0, forest()).expect("the Forest");
    let plains_id = on_battlefield(&engine, p0, plains()).expect("the Plains");
    assert!(types(&engine, forest_id).contains(TypeSet::CREATURE));
    let life = engine.state().players[0].life;

    kill(&mut engine, plains_id);
    assert_eq!(
        engine.state().players[0].life,
        life,
        "a land dying is no creature dying"
    );
    kill(&mut engine, forest_id);
    assert_eq!(
        engine.state().players[0].life,
        life + 1,
        "the Forest was a creature as it died"
    );
}

/// Moonlit Wake — {2}{W} enchantment: "Whenever a creature dies, you gain 1
/// life." The word the scenario turns on is "a creature", which names no
/// controller: the board carries an Elf on each side and both die, so the one
/// across the table has to pay the Wake's controller a life exactly as this
/// seat's own does — and the seat that lost the creature gains nothing. One
/// Llanowar Elf dies *before* the Wake resolves as the control, the same death
/// on the same board with nothing on the table to pay for it, and the pair of
/// readings 21 then 22 is what says each printed death paid exactly once.
#[test]
fn moonlit_wake_gains_a_life_for_a_creature_dying_on_either_side_of_the_table() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[moonlit_wake()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(mine.len(), 2, "two Elves on this side of the table");
    let (control, sacrifice) = (mine[0], mine[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // The control: the same death on the same board with no Moonlit Wake on
    // it. `bury` moves the card on the spot and hands no priority over, which
    // is the whole finding here — the Elf prints no dies trigger of its own.
    bury(&mut engine, &[control]);
    assert_eq!(
        all_on_battlefield(&engine, p0, llanowar_elves()).len(),
        1,
        "the control creature really left the battlefield"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "a creature dying with no Wake on the table pays nothing"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and pays the other seat nothing either"
    );

    // The card itself: {2}{W} off the three Plains, with the second Elf left
    // standing so that it can die once the Wake is out.
    cast_from_hand(&mut engine, p0, moonlit_wake());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, moonlit_wake()).is_some(),
        "the Wake resolved onto the battlefield"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "an enchantment entering is no creature dying"
    );

    kill(&mut engine, sacrifice);
    assert_eq!(
        engine.state().players[0].life,
        21,
        "\"whenever a creature dies, you gain 1 life\" — and the Elf that died was this seat's"
    );

    kill(&mut engine, theirs);
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the creature that died belonged to the other seat"
    );
    assert_eq!(
        engine.state().players[0].life,
        22,
        "\"a creature\" is not \"a creature you control\": a creature across the \
         table dying pays the Wake's controller just the same"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and \"you\" is the Wake's controller, not the seat that lost the creature"
    );
    assert!(
        on_battlefield(&engine, p0, moonlit_wake()).is_some(),
        "the Wake paid for two deaths and is still standing"
    );
}
