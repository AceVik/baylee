//! `cards/enchantments/mv_4/pursuit_of_knowledge.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::counters;

const P0: PlayerId = PlayerId::new(0);

fn pursuit_of_knowledge() -> CardIndex {
    card_index("7f5124a4-9ffd-475a-8b77-3ff59a9871a1")
}

/// Pursuit of Knowledge (`7f5124a4-9ffd-475a-8b77-3ff59a9871a1`) — {3}{W} Enchantment (Partial):
/// "Remove three study counters from this enchantment, Sacrifice this enchantment: Draw seven cards."
#[test]
fn pursuit_of_knowledge_removes_three_study_counters_and_sacrifices_to_draw_seven() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[pursuit_of_knowledge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let ench = on_battlefield(&engine, P0, pursuit_of_knowledge()).unwrap();
    assert!(!lk::offered(&engine, pursuit_of_knowledge(), P0, 0));
    engine
        .dev_state_mut(P0)
        .unwrap()
        .object_mut(ench)
        .unwrap()
        .counters
        .add(counters::STUDY, 2);
    engine.refresh_offer();
    assert!(!lk::offered(&engine, pursuit_of_knowledge(), P0, 0));
    engine
        .dev_state_mut(P0)
        .unwrap()
        .object_mut(ench)
        .unwrap()
        .counters
        .add(counters::STUDY, 1);
    engine.refresh_offer();

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    activate(&mut engine, P0, pursuit_of_knowledge(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, pursuit_of_knowledge()).is_none(),
        "Pursuit of Knowledge should be sacrificed"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before + 7
    );
}
