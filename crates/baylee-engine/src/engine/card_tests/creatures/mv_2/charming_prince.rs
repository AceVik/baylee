//! `cards/creatures/mv_2/charming_prince.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Charming Prince, "You gain 3 life."
#[test]
fn charming_prince_can_gain_three_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4604, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[charming_prince()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let life = engine.state().players[0].life;
    prince_enters_choosing(&mut engine, 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, life + 3);
}

/// Charming Prince, "Exile another target creature you own. Return it to the
/// battlefield under your control at the beginning of the next end step."
#[test]
fn charming_prince_can_blink_another_creature_until_the_end_step() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4605, plains())
        .battlefield(0, &[plains(), plains(), llanowar_elves()])
        .hand(0, &[charming_prince()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves");
    prince_enters_choosing(&mut engine, 2);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the Elves are the only other creature");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "exiled for now"
    );
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, llanowar_elves()).is_some()
    });
    assert!(
        matches!(engine.state().turn.step, Step::End),
        "back at the end step"
    );
}

/// Charming Prince, "Scry 2."
#[test]
fn charming_prince_can_scry_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4606, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[charming_prince()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    prince_enters_choosing(&mut engine, 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange { cards, .. } = engine.pending().clone() else {
        unreachable!("the scry arranges")
    };
    assert_eq!(cards.len(), 2, "two cards looked at");
    engine.apply(p0, look_answer(&cards, &[])).unwrap();
    pass_until(&mut engine, stack_is_empty);
}
