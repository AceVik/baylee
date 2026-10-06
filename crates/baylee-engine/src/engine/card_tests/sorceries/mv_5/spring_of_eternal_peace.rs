//! `cards/sorceries/mv_5/spring_of_eternal_peace.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Spring of Eternal Peace` is a sorcery costing `{3}{G}{G}` under `Coverage::Implemented`.
/// It prints "You gain 8 life."
/// When cast from hand off five Forests, it resolves without needing targets and increases
/// its controller's life total by 8 before being placed into the graveyard.
#[test]
fn spring_of_eternal_peace_gains_eight_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[spring_of_eternal_peace()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, spring_of_eternal_peace());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        28,
        "Spring of Eternal Peace increased life total by 8"
    );
    assert!(
        in_graveyard(&engine, p0, spring_of_eternal_peace()).is_some(),
        "resolved sorcery is in caster's graveyard"
    );
}
