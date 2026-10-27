//! `cards/creatures/mv_6/arcanis_the_omnipotent.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn arcanis_the_omnipotent_taps_to_draw_three_and_returns_itself_to_hand() {
    let p0 = PlayerId::new(0);
    let arcanis = card_index("8a7183cc-161c-444d-a889-a17519c8061b");
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[arcanis, island(), island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = hand_size(&engine, p0);
    activate(&mut engine, p0, arcanis, 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(hand_size(&engine, p0), hand_before + 3);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, arcanis, 1);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, arcanis).is_none());
    assert!(in_hand(&engine, p0, arcanis).is_some());
}
