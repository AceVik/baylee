//! `cards/enchantments/mv_1/seal_of_fire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Seal of Fire` (`Coverage::Implemented`):
/// "Sacrifice this enchantment: It deals 2 damage to any target."
///
/// Verifies that activating `Seal of Fire` sacrifices itself and deals 2 damage
/// to the chosen target player.
#[test]
fn seal_of_fire_sacrifices_to_deal_two_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1324, forest())
        .battlefield(0, &[seal_of_fire()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert_eq!(engine.state().players[1].life, 20);

    activate(&mut engine, p0, seal_of_fire(), 0);
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Seal of Fire, got {:?}",
            engine.pending()
        )
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

    assert_eq!(
        engine.state().players[1].life,
        18,
        "dealt 2 damage to opponent"
    );
    assert!(
        in_graveyard(&engine, p0, seal_of_fire()).is_some(),
        "Seal of Fire is in graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, seal_of_fire()).is_none(),
        "Seal of Fire left the battlefield"
    );
}
