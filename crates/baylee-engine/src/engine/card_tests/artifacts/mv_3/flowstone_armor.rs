//! `cards/artifacts/mv_3/flowstone_armor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn flowstone_armor_gives_plus_one_minus_one_while_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                card_index("81f44bac-d28c-4dc1-922c-f4ebda817276"),
                forest(),
                forest(),
                forest(),
                index::HILL_GIANT,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let armor = on_battlefield(
        &engine,
        p0,
        card_index("81f44bac-d28c-4dc1-922c-f4ebda817276"),
    )
    .expect("Flowstone Armor");
    let giant = on_battlefield(&engine, p0, index::HILL_GIANT).expect("Hill Giant");
    assert_eq!(pt(&engine, giant), (3, 3));
    tap_all_mana(&mut engine, p0);
    activate(
        &mut engine,
        p0,
        card_index("81f44bac-d28c-4dc1-922c-f4ebda817276"),
        1,
    );
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, armor));
    assert_eq!(pt(&engine, giant), (4, 2));
}
