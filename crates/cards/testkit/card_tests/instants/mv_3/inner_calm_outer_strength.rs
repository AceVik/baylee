//! `cards/instants/mv_3/inner_calm_outer_strength.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Inner Calm, Outer Strength (`Coverage::Implemented`): "Target creature
/// gets +X/+X until end of turn, where X is the number of cards in your hand."
///
/// The spell card is on the stack during resolution rather than in hand, so
/// two remaining cards in hand grant +2/+2. The test targets a 1/1 Llanowar
/// Elves and confirms its projected power and toughness become 3/3.
#[test]
fn inner_calm_outer_strength_pumps_target_creature_by_cards_in_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(42, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .hand(0, &[inner_calm_outer_strength(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is deployed");
    assert_eq!(pt(&engine, elf), (1, 1), "starts as a 1/1 creature");

    cast_from_hand(&mut engine, p0, inner_calm_outer_strength());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&elf),
        "target creature — the Elf qualifies"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, elf),
        (3, 3),
        "two cards remaining in hand give +2/+2, turning the 1/1 into a 3/3"
    );
    assert!(
        in_graveyard(&engine, p0, inner_calm_outer_strength()).is_some(),
        "resolved spell moves to the graveyard"
    );
}
