//! `cards/creatures/mv_2/rootwater_thief.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn rootwater_thief_gains_flying_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[card_index("68abaca6-a86a-46e3-b5b8-9860bb6425d2"), island()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let thief = on_battlefield(
        &engine,
        p0,
        card_index("68abaca6-a86a-46e3-b5b8-9860bb6425d2"),
    )
    .expect("Rootwater Thief");
    assert!(!keywords(&engine, thief).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p0);
    activate(
        &mut engine,
        p0,
        card_index("68abaca6-a86a-46e3-b5b8-9860bb6425d2"),
        0,
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, thief).contains(KeywordSet::FLYING));
}
