//! `cards/creatures/mv_4/daru_warchief.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn daru_warchief_reduces_soldier_spell_costs_and_buffs_soldiers() {
    let warchief = card_index("3fce53fb-6914-4f43-b46b-18ca32ab21b2");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[warchief, index::PEARLED_UNICORN, plains(), plains()])
        .battlefield(1, &[index::STANDING_TROOPS])
        .hand(0, &[index::STANDING_TROOPS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let warchief_id = on_battlefield(&engine, p0, warchief).unwrap();
    let unicorn_id = on_battlefield(&engine, p0, index::PEARLED_UNICORN).unwrap();
    let opp_troops_id = on_battlefield(&engine, p1, index::STANDING_TROOPS).unwrap();

    assert_eq!(
        pt(&engine, warchief_id),
        (2, 3),
        "1/1 Daru Warchief gets +1/+2"
    );
    assert_eq!(pt(&engine, unicorn_id), (2, 2), "non-Soldier is unaffected");
    assert_eq!(
        pt(&engine, opp_troops_id),
        (1, 4),
        "opponent's Soldier is unaffected"
    );

    // Standing Troops ({2}{W}) costs {1}{W} with Daru Warchief.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, index::STANDING_TROOPS);
    pass_until(&mut engine, stack_is_empty);

    let my_troops_id = on_battlefield(&engine, p0, index::STANDING_TROOPS).unwrap();
    assert_eq!(pt(&engine, my_troops_id), (2, 6));
}
