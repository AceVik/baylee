//! `cards/instants/mv_2/blood_lust.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blood Lust (55a253f9-f44d-4898-af83-fea68b7b279d) — "If target creature has
/// toughness 5 or greater, it gets +4/-4 until end of turn. Otherwise, it gets
/// +4/-X until end of turn, where X is its toughness minus 1."
#[test]
fn blood_lust_gives_plus_4_minus_4_to_toughness_5_or_leaves_1_toughness_below_5() {
    let p0 = PlayerId::new(0);
    let blood_lust = card_index("55a253f9-f44d-4898-af83-fea68b7b279d");
    // Hill Giant (3/3) and Obsianus Golem (4/6).
    let hill_giant = card_index("342199e0-15b6-4824-83da-25caef2592b3");
    let obsianus_golem = card_index("ac41171e-c454-49e9-9004-c082ae099630");
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[blood_lust, blood_lust])
        .battlefield(
            0,
            &[
                hill_giant,
                obsianus_golem,
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let giant_id = on_battlefield(&engine, p0, hill_giant).unwrap();
    let golem_id = on_battlefield(&engine, p0, obsianus_golem).unwrap();
    assert_eq!(pt(&engine, giant_id), (3, 3));
    assert_eq!(pt(&engine, golem_id), (4, 6));

    tap_all_mana(&mut engine, p0);
    cast_from_hand(&mut engine, p0, blood_lust);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![giant_id],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, giant_id),
        (7, 1),
        "3/3 creature has toughness < 5, so gets +4/-(3-1) = +4/-2 -> 7/1"
    );

    cast_from_hand(&mut engine, p0, blood_lust);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![golem_id],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, golem_id),
        (8, 2),
        "4/6 creature has toughness >= 5, so gets +4/-4 -> 8/2"
    );
}
