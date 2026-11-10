//! `cards/instants/mv_2/consult_the_star_charts.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "Look at the top X cards of your library, where X is the number of lands
/// you control. Put one of those cards into your hand." — four lands, four
/// cards, one kept, when the kicker is declined.
#[test]
fn consult_the_star_charts_unkicked_keeps_one_of_as_many_as_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[consult_the_star_charts()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, consult_the_star_charts());
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::Kicker,
            ..
        }
    ));
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    keep_first(&mut engine, p0, 4, 1);
}

/// "If this spell was kicked, put two of those cards into your hand
/// instead."
#[test]
fn consult_the_star_charts_kicked_keeps_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[consult_the_star_charts()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, consult_the_star_charts());
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    keep_first(&mut engine, p0, 5, 2);
}
