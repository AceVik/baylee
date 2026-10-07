//! `cards/instants/mv_1/swords_to_plowshares.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// And the other way round: the Elves are exiled in response, so the copy
/// has nothing to copy, and the Ritual is countered all the same. Nothing
/// else becomes the copy's target when its own is gone.
#[test]
fn three_steps_ahead_still_counters_when_its_copy_target_is_gone() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let (mut engine, ritual, elves) =
        three_steps_ahead_of_a_ritual(&[plains()], &[swords_to_plowshares()]);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, swords_to_plowshares());
    let _ = aim_at(&mut engine, p1, elves);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(ritual).map(|o| o.zone),
        Some(Zone::Graveyard)
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0, "countered");
    assert!(tokens_of(&engine, p0).is_empty(), "nothing was copied");
}
