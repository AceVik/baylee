//! `cards/enchantments/mv_5/call_to_the_grave.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn call_to_the_grave_forces_non_zombie_sacrifice_on_upkeep_and_sacrifices_itself_when_board_empty()
{
    let call = card_index("db5a4c25-5ae5-4a04-be79-bdee39b9152c");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[call, index::GRIZZLY_BEARS])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);

    // Turn 1 upkeep (p0): p0 must sacrifice non-Zombie Grizzly Bears.
    run_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    assert!(in_graveyard(&engine, p0, index::GRIZZLY_BEARS).is_some());
    assert!(
        on_battlefield(&engine, p0, call).is_some(),
        "Call to the Grave stays while p1 still controls Hill Giant"
    );

    // Turn 2 upkeep (p1): p1 must sacrifice non-Zombie Hill Giant.
    run_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    assert!(in_graveyard(&engine, p1, index::HILL_GIANT).is_some());

    // At the beginning of Turn 2's end step, no creatures are on the battlefield, so Call to the Grave sacrifices itself.
    run_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    assert!(on_battlefield(&engine, p0, call).is_none());
    assert!(in_graveyard(&engine, p0, call).is_some());
}
