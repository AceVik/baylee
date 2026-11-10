//! `cards/artifacts/mv_2/aeolipile.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Aeolipile` prints `{{1}}, {{T}}, Sacrifice this artifact: It deals 2 damage to any target.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Aeolipile` and a `forest()`.
/// Floating one mana pays the activation cost to target seat 1 via `Pending::ChooseTargets`,
/// sacrificing `Aeolipile` and dealing 2 damage to the opponent upon resolution.
#[test]
fn aeolipile_sacrifices_to_deal_two_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), aeolipile()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana_but(&mut engine, p0, Some(aeolipile()));
    activate(&mut engine, p0, aeolipile(), 0);

    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(player_options.contains(&p1), "opponent is a legal target");

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("targeted opponent");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, aeolipile()).is_some(),
        "`Aeolipile` was sacrificed"
    );
    assert_eq!(engine.state().players[1].life, 18, "opponent took 2 damage");
}
