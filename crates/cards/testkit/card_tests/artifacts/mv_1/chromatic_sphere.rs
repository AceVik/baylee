//! `cards/artifacts/mv_1/chromatic_sphere.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Chromatic Sphere` prints `{{1}}, {{T}}, Sacrifice this artifact: Add one mana of any color. Draw a card.`
/// with `Coverage::Implemented`.
/// In this scenario, seat 0 controls a `Forest` and `Chromatic Sphere`.
/// One green mana pays the cost and the artifact is sacrificed immediately.
/// The ability uses the stack; its color choice, mana and draw occur on resolution.
#[test]
fn chromatic_sphere_adds_chosen_mana_and_draws_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), chromatic_sphere()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let _sphere = on_battlefield(&engine, p0, chromatic_sphere()).expect("sphere on battlefield");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    tap_all_mana_but(&mut engine, p0, Some(chromatic_sphere()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, chromatic_sphere(), 0);
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 1);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("chose blue mana");

    pass_until(&mut engine, stack_is_empty);
    assert!(stack_is_empty(&engine), "the ability finished resolving");
    assert!(
        in_graveyard(&engine, p0, chromatic_sphere()).is_some(),
        "sphere was sacrificed"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "drew a card"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1, "added one blue mana");
    assert_eq!(pool.total(), 1, "exactly one mana in pool");
}
