//! `cards/creatures/mv_7/thorn_elemental.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn thorn_elemental() -> CardIndex {
    card_index("9cdf840d-314e-46ef-9c04-74f145d9e65b")
}

#[test]
fn thorn_elemental_casts_for_seven_and_fights_as_a_seven_seven() {
    // Unblocked: casts off {5}{G}{G} (with 6 lands insufficient) and deals 7
    // combat damage to the defending player.
    let mut engine = table7(&lots(forest(), 7), &[thorn_elemental()], &[]);
    let elemental = cast_and_resolve(&mut engine, thorn_elemental());
    assert_eq!(pt(&engine, elemental), (7, 7));

    // Next turn cycle: attack unblocked for 7 damage.
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::Upkeep
    });
    reach_main_phase(&mut engine, p0);
    fight(&mut engine, &[elemental], &[]);
    assert_eq!(engine.state().players[1].life, 13);

    // Blocked: because optional unblocked combat-damage assignment is not in
    // the DSL (`Coverage::Partial`), a 2/2 blocker absorbs Thorn Elemental's
    // 7 combat damage and dies while seat 1 stays at 20 life.
    let mut blocked = Duel::new(SEED, forest())
        .battlefield(0, &[thorn_elemental()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut blocked);
    reach_main_phase(&mut blocked, p0);
    let attacker = on_battlefield(&blocked, p0, thorn_elemental()).unwrap();
    let bear = on_battlefield(&blocked, p1, grizzly_bears()).unwrap();
    fight(&mut blocked, &[attacker], &[(bear, attacker)]);
    assert_eq!(blocked.state().players[1].life, 20);
    assert!(in_graveyard(&blocked, p1, grizzly_bears()).is_some());
    assert!(on_battlefield(&blocked, p0, thorn_elemental()).is_some());
}
