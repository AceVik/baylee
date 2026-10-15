//! `cards/creatures/artifacts/mv_4/scrapyard_steelbreaker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn scrapyard_steelbreaker() -> CardIndex {
    card_index("172e7250-94a4-466f-9438-159174925012")
}

/// Scrapyard Steelbreaker — {3}{R} 3/4: "{1}, Sacrifice another artifact:
/// This creature gets +2/+1 until end of turn." With a Sol Ring and a Forest
/// the Ring is sacrificed and the Steelbreaker is a 5/5; the boost is gone
/// next turn.
#[test]
fn scrapyard_steelbreaker_sacrifices_another_artifact_for_a_boost() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[scrapyard_steelbreaker(), sol_ring(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let breaker = on_battlefield(&engine, p0, scrapyard_steelbreaker()).expect("seated");
    assert_eq!(pt(&engine, breaker), (3, 4));

    let forests = all_on_battlefield(&engine, p0, forest());
    tap_mana_where(&mut engine, p0, |id| forests.contains(&id));
    activate(&mut engine, p0, scrapyard_steelbreaker(), 0);
    answer_until_settled(&mut engine);

    assert!(
        in_graveyard(&engine, p0, sol_ring()).is_some(),
        "sacrificed"
    );
    assert_eq!(pt(&engine, breaker), (5, 5));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0, "{{1}} paid");
    pass_until(&mut engine, |e| e.state().turn.number >= 2);
    assert_eq!(pt(&engine, breaker), (3, 4), "until end of turn");
}

/// "Another": the Steelbreaker is an artifact itself, but with no other one
/// on the board the ability is not offered even with {1} floating.
#[test]
fn scrapyard_steelbreaker_cannot_sacrifice_itself() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[scrapyard_steelbreaker(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let breaker = on_battlefield(&engine, p0, scrapyard_steelbreaker()).expect("seated");
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert!(!priority_offer(&engine).abilities.contains(&(breaker, 0)));
}
