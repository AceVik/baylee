//! `cards/artifacts/mv_2/fellwar_stone.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fellwar Stone reads the colours off the lands an *opponent* controls.
/// Reflecting Pool's side of that effect had a test; this side had a card
/// (Exotic Orchard) and none — and the two differ by one comparison, so a
/// sign error there would have produced a Stone that reads your own lands
/// and passed every test in the suite.
#[test]
fn fellwar_stone_reads_the_opponents_lands_and_not_your_own() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(32, forest())
        .battlefield(0, &[forest(), fellwar_stone()])
        .battlefield(1, &[badlands()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, fellwar_stone(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected a colour choice, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![ManaColor::Black, ManaColor::Red],
        "the opponent's Badlands — your own Forest is not an option"
    );
}
