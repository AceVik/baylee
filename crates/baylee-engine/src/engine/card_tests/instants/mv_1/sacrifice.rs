//! `cards/instants/mv_1/sacrifice.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sacrifice — {B}: "As an additional cost to cast this spell, sacrifice a
/// creature. Add an amount of {B} equal to the sacrificed creature's mana
/// value." The amount is what the cost recorded as it was paid: Ondu Cleric
/// ({1}{W}) is two black mana, and the {B} that paid for the spell is gone.
#[test]
fn sacrifice_adds_black_mana_equal_to_the_sacrificed_creatures_mana_value() {
    let p0 = PlayerId::new(0);
    let sacrifice = card_index("068b3692-411b-44d4-a7e9-005262760cfc");
    let mut engine = Duel::new(12, forest())
        .battlefield(0, &[swamp(), ondu_cleric()])
        .hand(0, &[sacrifice])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let cleric = on_battlefield(&engine, p0, ondu_cleric()).expect("the cleric is out");
    cast_from_hand(&mut engine, p0, sacrifice);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("the sacrifice is asked at cast, got {:?}", engine.pending())
    };
    assert_eq!(options, vec![cleric]);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: options })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, ondu_cleric()).is_none());
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(baylee_core::mana::ManaColor::Black),
        2
    );
}
