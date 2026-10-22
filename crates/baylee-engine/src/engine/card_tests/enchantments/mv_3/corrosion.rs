//! `cards/enchantments/mv_3/corrosion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn corrosion() -> CardIndex {
    card_index("3d3fd8a2-78d4-4f52-8fcd-0c6fa93f0043")
}

#[test]
fn corrosion_upkeep_puts_rust_counter_on_opponents_artifacts() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), mountain(), swamp()])
        .battlefield(1, &[sol_ring()])
        .hand(0, &[corrosion()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let opp_ring = on_battlefield(&engine, p1, sol_ring()).unwrap();
    cast_from_hand(&mut engine, p0, corrosion());
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, opp_ring, baylee_cards_dsl::counters::RUST),
        1
    );
}
