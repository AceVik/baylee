//! `cards/enchantments/mv_3/rhystic_study.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Rhystic Study` is an enchantment costing `{2}{U}` under `Coverage::Implemented`.
/// It prints "Whenever an opponent casts a spell, you may draw a card unless that player pays {1}."
/// When an opponent casts a spell, the triggered ability prompts the opponent with `YesNoPrompt::PayTax`,
/// and when the opponent declines to pay, its controller draws a card.
#[test]
fn rhystic_study_triggers_on_opponent_cast_and_draws_when_tax_declined() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[rhystic_study()])
        .hand(0, &[])
        .battlefield(1, &[forest()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);

    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, llanowar_elves());

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { .. },
                ..
            } if *player == p1
        )
    });

    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "controller drew one card after opponent declined to pay the tax"
    );
}
