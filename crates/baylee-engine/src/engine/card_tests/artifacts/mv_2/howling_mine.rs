//! `cards/artifacts/mv_2/howling_mine.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Howling Mine: "At the beginning of each player's draw step, if this
/// artifact is untapped, that player draws an additional card." The player
/// going first skips their own turn-1 draw step entirely (CR 103.8a), so
/// the first draw this reaches is p1's. The negative half taps the Mine
/// during p0's own turn-3 upkeep — strictly after that turn's untap step,
/// which would otherwise undo a tap set any earlier.
#[test]
fn howling_mine_doubles_the_draw_only_while_untapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[howling_mine()])
        .start();
    keep_mulligans(&mut engine);
    let baseline0 = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let baseline1 = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        baseline1 + 2,
        "p1's own first draw, doubled by the untapped Mine"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && stack_is_empty(e)
    });
    let mine = on_battlefield(&engine, p0, howling_mine()).expect("seated");
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .object_mut(mine)
        .expect("seated")
        .status
        .insert(Status::TAPPED);
    engine.refresh_offer();

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain) && e.state().turn.active == p0
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        baseline0 + 1,
        "p0's own first draw, single — the Mine is tapped"
    );
}
