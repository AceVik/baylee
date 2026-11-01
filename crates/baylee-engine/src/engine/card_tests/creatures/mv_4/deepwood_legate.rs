//! `cards/creatures/mv_4/deepwood_legate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn subject_card() -> CardIndex {
    card_index("414eb407-dd39-41d2-bf25-a7a77ff35cb4")
}

/// Deepwood Legate — {3}{B} 1/1 Creature — Shade:
/// "{B}: This creature gets +1/+1 until end of turn." (Partial: conditional free cast unimplemented).
#[test]
fn deepwood_legate_pumps_for_black_mana_until_end_of_turn() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[subject_card()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, subject_card());
    pass_until(&mut engine, stack_is_empty);

    let legate = on_battlefield(&engine, P0, subject_card()).unwrap();
    assert_eq!(pt(&engine, legate), (1, 1));

    activate(&mut engine, P0, subject_card(), 0);
    pass_until(&mut engine, stack_is_empty);
    activate(&mut engine, P0, subject_card(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, legate), (3, 3));

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, legate), (1, 1));
}
