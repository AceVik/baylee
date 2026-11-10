//! `cards/creatures/mv_7/jumbo_cactuar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn jumbo_cactuar() -> CardIndex {
    card_index("e0bdfbb4-3060-4492-bd9b-0c47e719400a")
}

/// Jumbo Cactuar — a 1/7 for {5}{G}{G}.
#[test]
fn jumbo_cactuar_costs_seven_and_is_a_one_seven() {
    let mut engine = table7(&lots(forest(), 7), &[jumbo_cactuar()], &[]);
    let cactuar = cast_and_resolve(&mut engine, jumbo_cactuar());
    assert_body(&engine, cactuar, (1, 7), KeywordSet::default());
}

/// "10,000 Needles — Whenever this creature attacks, it gets +9999/+0 until
/// end of turn": unblocked it deals 10,000.
#[test]
fn jumbo_cactuar_gets_9999_when_it_attacks() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[jumbo_cactuar()])
        .life(1, 20_000)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let cactuar = on_battlefield(&engine, p0, jumbo_cactuar()).expect("out");
    assert_eq!(life_of(&engine, 1), 20_000);
    declare_attack(&mut engine, &[cactuar], &[]);
    assert_eq!(pt(&engine, cactuar), (10_000, 7), "the trigger resolved");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(life_of(&engine, 1), 10_000, "10,000 damage");
}
