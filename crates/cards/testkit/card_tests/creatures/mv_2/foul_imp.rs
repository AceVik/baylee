//! `cards/creatures/mv_2/foul_imp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Foul Imp` prints `KeywordSet::FLYING` and an enters-the-battlefield trigger:
/// "When this creature enters, you lose 2 life" under `Coverage::Implemented`.
/// Casting the spell from hand off two Swamps resolves the creature and triggers its entry ability,
/// reducing the caster's life total from 20 to 18 while placing the flyer on the battlefield.
#[test]
fn foul_imp_causes_two_life_loss_on_entry() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1622, forest())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[foul_imp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert_eq!(engine.state().players[0].life, 20);

    cast_from_hand(&mut engine, p0, foul_imp());
    pass_until(&mut engine, stack_is_empty);

    let imp = on_battlefield(&engine, p0, foul_imp()).expect("imp entered the battlefield");
    assert!(keywords(&engine, imp).contains(KeywordSet::FLYING));
    assert_eq!(engine.state().players[0].life, 18, "controller lost 2 life");
}
