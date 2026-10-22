//! `cards/creatures/mv_5/morinfen.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::MORINFEN;

/// `Morinfen` ({3}{B}{B}, Legendary Creature — Phyrexian Horror 5/4) has flying
/// and cumulative upkeep—Pay 1 life.
#[test]
fn morinfen_cumulative_upkeep_pays_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp()).battlefield(0, &[MORINFEN]).start();
    keep_mulligans(&mut engine);

    let morinfen = on_battlefield(&engine, p0, MORINFEN).unwrap();
    assert!(keywords(&engine, morinfen).contains(KeywordSet::FLYING));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, MORINFEN).is_some());
    assert_eq!(engine.state().players[0].life, 19);
}
