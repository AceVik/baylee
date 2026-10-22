//! `cards/creatures/mv_5/gallowbraid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GALLOWBRAID;

/// `Gallowbraid` ({3}{B}{B}, Legendary Creature — Phyrexian Horror 5/5) has
/// trample and cumulative upkeep—Pay 1 life.
#[test]
fn gallowbraid_pays_cumulative_upkeep_life_and_has_trample() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[GALLOWBRAID])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, GALLOWBRAID);
    pass_until(&mut engine, stack_is_empty);

    let gallowbraid = on_battlefield(&engine, p0, GALLOWBRAID).unwrap();
    assert!(keywords(&engine, gallowbraid).contains(KeywordSet::TRAMPLE));

    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, GALLOWBRAID).is_some());
    assert_eq!(engine.state().players[0].life, 19);
}
