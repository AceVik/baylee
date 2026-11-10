//! `cards/creatures/mv_2/femeref_enchantress.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Femeref Enchantress` prints a triggered ability under `Coverage::Implemented`:
/// "Whenever an enchantment is put into a graveyard from the battlefield, draw a card."
/// When an enchantment (`Seal of Fire`) is sacrificed to deal damage, its departure triggers
/// the Enchantress to draw a card, increasing the hand size by one upon resolution.
#[test]
fn femeref_enchantress_draws_when_enchantment_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let seal = card_index("348a345e-4639-41ca-b015-a5d43459eb64");
    let mut engine = Duel::new(1618, forest())
        .battlefield(0, &[femeref_enchantress(), seal])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, seal, 0);
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("expected target choice for Seal of Fire");
    };
    assert!(player_options.contains(&p1));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, seal).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "a card was drawn from the enchantment death trigger"
    );
    assert_eq!(engine.state().players[1].life, 18);
}
