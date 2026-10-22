//! `cards/enchantments/mv_5/malignant_growth.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::counters;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "b3235d21-ce52-4a99-a2fe-99b0fe7cc8bc"
fn malignant_growth() -> CardIndex {
    card_index("b3235d21-ce52-4a99-a2fe-99b0fe7cc8bc")
}

#[test]
fn malignant_growth_adds_growth_counter_at_upkeep_and_draws_and_damages_opponent_in_draw_step() {
    let mut engine = lk::game(SEED, &[island()], &[], &[malignant_growth()], &[]);
    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Colorless, 3),
            (ManaColor::Green, 1),
            (ManaColor::Blue, 1),
        ],
    );
    cast_with_floating(&mut engine, P0, malignant_growth());
    pass_until(&mut engine, stack_is_empty);
    let growth = lk::obj(&engine, P0, malignant_growth());

    // Advance to P1's turn, then back to P0's upkeep so Malignant Growth gets a growth counter.
    reach_their_main_phase(&mut engine, P1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    tap_all_mana(&mut engine, P0);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P0),
    );
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    reach_their_main_phase(&mut engine, P0);

    assert_eq!(
        engine
            .state()
            .object(growth)
            .unwrap()
            .counters
            .get(counters::GROWTH),
        1
    );

    let p1_hand_before = engine.state().zones.list(ZoneLocation::Hand(P1)).len();
    reach_their_main_phase(&mut engine, P1);
    let p1_hand_after = engine.state().zones.list(ZoneLocation::Hand(P1)).len();

    assert_eq!(
        p1_hand_after,
        p1_hand_before + 2,
        "P1 drew their normal draw-step card plus 1 extra card from Malignant Growth"
    );
    assert_eq!(
        engine.state().players[1].life,
        19,
        "Malignant Growth dealt 1 damage to P1 for the 1 growth counter"
    );
}
