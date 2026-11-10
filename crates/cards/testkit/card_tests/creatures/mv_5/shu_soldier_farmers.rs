//! `cards/creatures/mv_5/shu_soldier_farmers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "174cf7cd-3e8c-4f90-abc1-a78a0ce832d2"

/// Shu Soldier-Farmers is a {4}{W} 2/4 printing one sentence: "When this
/// creature enters, you gain 4 life." The scenario plays it out of five Plains
/// and reads both halves where the rules put them — while the spell sits on the
/// stack no life has moved, because an enters trigger waits for the permanent
/// (CR 603.6a) and a card implemented as a cast-time heal would already read
/// 24 here. Once it has landed, the body, the four life and the mana are all
/// asserted together, and the opponent's untouched total is the control that
/// says "you" is the controller and not the table.
#[test]
fn shu_soldier_farmers_gains_four_life_for_its_controller_when_it_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .hand(0, &[shu_soldier_farmers()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert_eq!(
        (
            engine.state().players[0].life,
            engine.state().players[1].life
        ),
        (20, 20),
        "nothing has entered yet"
    );

    cast_from_hand(&mut engine, p0, shu_soldier_farmers());
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the life belongs to the entering trigger and not to the casting of the spell"
    );

    pass_until(&mut engine, |e| {
        stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. })
    });

    let farmers = on_battlefield(&engine, p0, shu_soldier_farmers())
        .expect("the Soldier-Farmers resolved onto the battlefield");
    assert_eq!(pt(&engine, farmers), (2, 4), "the printed 2/4 body");
    assert!(
        types(&engine, farmers).contains(TypeSet::CREATURE),
        "and it arrives as the creature it prints"
    );
    assert_eq!(
        engine.state().players[0].life,
        24,
        "\"you gain 4 life\" — four, once, for the seat that cast it"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and never for the opponent: \"you\" is the controller"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "five Plains paid {{4}}{{W}} and left nothing floating"
    );
}
