//! `cards/creatures/artifacts/mv_2/glaze_fiend.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn glaze_fiend() -> CardIndex {
    card_index("6860cde7-ee8e-4a4d-baed-9dc4a1220e36")
}

/// Glaze Fiend: a 0/1 with "Flying": only a flier may block it.
#[test]
fn glaze_fiend_has_flying() {
    only_a_flier_may_block(glaze_fiend());
}

/// "Whenever another artifact you control enters, this creature gets +2/+2
/// until end of turn": a Gold Myr arriving makes it a 2/3, and the pump ends
/// with the turn.
#[test]
fn glaze_fiend_gets_plus_two_plus_two_when_another_artifact_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[gold_myr()], &[glaze_fiend(), forest(), forest()], &[]);
    let fiend = on_battlefield(&engine, p0, glaze_fiend()).expect("seated");
    assert_eq!(pt(&engine, fiend), (0, 1));
    cast_and_land(&mut engine, gold_myr());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, fiend), (2, 3), "+2/+2");
    to_my_next_turn(&mut engine);
    assert_eq!(pt(&engine, fiend), (0, 1), "until end of turn");
}

/// A non-artifact entering does nothing: the Bears are no artifact.
#[test]
fn glaze_fiend_ignores_a_nonartifact_entering() {
    let p0 = PlayerId::new(0);
    let mut engine = table(
        &[grizzly_bears()],
        &[glaze_fiend(), forest(), forest()],
        &[],
    );
    let fiend = on_battlefield(&engine, p0, glaze_fiend()).expect("seated");
    cast_and_land(&mut engine, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, fiend), (0, 1));
}

/// "Another": the Fiend itself entering from hand pumps nothing.
#[test]
fn glaze_fiend_itself_entering_gets_no_pump() {
    let mut engine = table(&[glaze_fiend()], &[swamp(), forest()], &[]);
    let fiend = cast_and_land(&mut engine, glaze_fiend());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, fiend), (0, 1));
}
