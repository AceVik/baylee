//! `cards/sorceries/mv_4/rejuvenate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Rejuvenate` is a sorcery costing `{3}{G}` under `Coverage::Implemented` that gives its caster 6 life.
/// When cast from hand off four Forests, it resolves without needing targets and increases its controller's
/// life total by 6 before going to the graveyard.
#[test]
fn rejuvenate_casts_and_gains_six_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[rejuvenate()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, rejuvenate());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        26,
        "Rejuvenate gained 6 life for its controller"
    );
    assert!(
        in_graveyard(&engine, p0, rejuvenate()).is_some(),
        "resolved Rejuvenate sits in owner's graveyard"
    );
}
