//! `cards/enchantments/mv_4/parallax_tide.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn parallax_tide_exiles_lands_and_returns_them_when_leaving_battlefield() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .hand(
            0,
            &[
                card_index("e36a92be-537d-4a63-858a-afaab064fbf7"),
                index::DISENCHANT,
            ],
        )
        .battlefield(
            0,
            &[island(), island(), plains(), plains(), plains(), plains()],
        )
        .battlefield(1, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(
        &mut engine,
        p0,
        card_index("e36a92be-537d-4a63-858a-afaab064fbf7"),
    );
    pass_until(&mut engine, stack_is_empty);

    let opp_land = on_battlefield(&engine, p1, mountain()).expect("opponent Mountain");
    activate(
        &mut engine,
        p0,
        card_index("e36a92be-537d-4a63-858a-afaab064fbf7"),
        1,
    );
    aim_at(&mut engine, p0, opp_land);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, mountain()).is_none(),
        "Mountain exiled by Parallax Tide"
    );

    let tide = on_battlefield(
        &engine,
        p0,
        card_index("e36a92be-537d-4a63-858a-afaab064fbf7"),
    )
    .expect("Parallax Tide");
    cast_with_floating(&mut engine, p0, index::DISENCHANT);
    aim_at(&mut engine, p0, tide);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, mountain()).is_some(),
        "Mountain returned to battlefield when Parallax Tide left"
    );
}
