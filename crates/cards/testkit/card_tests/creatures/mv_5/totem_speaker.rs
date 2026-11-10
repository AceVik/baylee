//! `cards/creatures/mv_5/totem_speaker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Totem Speaker — {4}{G} — Creature — Elf Druid — 3/3: "Whenever a Beast
/// enters, you may gain 3 life."
///
/// The trigger is the whole card, so the board carries the Speaker plus a
/// Beast to arrive and a creature that is no Beast. Ravenous Baloth is the
/// Beast, and the Llanowar Elves beside it is the control: a trigger that had
/// fired on any creature entering would stand at 26 life and the Elf's own
/// entry would be the sentence that said so. Five Forests pay the Baloth's
/// {2}{G}{G} and leave exactly the one green the Elf is then cast with, so both
/// entries are real casts out of a pool that really held the mana.
#[test]
fn totem_speaker_gains_three_life_for_a_beast_and_nothing_for_an_elf() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                totem_speaker(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[ravenous_baloth(), llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a main phase of its own"
    );

    assert!(
        on_battlefield(&engine, p0, totem_speaker()).is_some(),
        "the Speaker is on the table to watch what arrives"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "nothing has entered and nothing has been gained"
    );

    // The Beast. Five Forests become five green, the Baloth's {2}{G}{G} spends
    // four of them and leaves the fifth floating for the Elf below.
    cast_from_hand(&mut engine, p0, ravenous_baloth());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "five tapped Forests less the {{2}}{{G}}{{G}} the Baloth costs"
    );
    pass_until(&mut engine, |e| {
        at_rest(e, p0) && e.state().players[0].life == 23
    });
    assert!(
        on_battlefield(&engine, p0, ravenous_baloth()).is_some(),
        "the Beast really entered, which is the event the trigger is written about"
    );
    assert_eq!(
        engine.state().players[0].life,
        23,
        "\"you may gain 3 life\" — taken once, for one Beast"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life belongs to the Speaker's controller, not to the opponent"
    );

    // The control: a creature that is no Beast. The green still floating pays
    // for it, so this is a real entry too and not a refused cast.
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, |e| {
        at_rest(e, p0) && e.state().players[0].life == 23
    });
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the Elf resolved, so a creature did enter under the Speaker's watch"
    );
    assert_eq!(
        engine.state().players[0].life,
        23,
        "an Elf is no Beast: a trigger that read \"any creature\" would be at 26 here"
    );
    assert!(
        on_battlefield(&engine, p0, totem_speaker()).is_some(),
        "the Speaker was not consumed by its own trigger"
    );
}
