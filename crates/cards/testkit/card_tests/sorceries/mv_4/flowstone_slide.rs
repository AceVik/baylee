//! `cards/sorceries/mv_4/flowstone_slide.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn flowstone_slide_gives_all_creatures_plus_x_minus_x() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[card_index("ad779760-1197-4248-8e21-7a048122640f")])
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                index::HILL_GIANT,
            ],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let giant = on_battlefield(&engine, p0, index::HILL_GIANT).expect("Hill Giant");
    tap_all_mana(&mut engine, p0);
    cast_with_floating(
        &mut engine,
        p0,
        card_index("ad779760-1197-4248-8e21-7a048122640f"),
    );
    engine
        .apply(p0, PlayerAction::ChooseNumber(2))
        .expect("announce X=2");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, giant), (5, 1));
    assert_eq!(on_battlefield(&engine, p1, index::GRIZZLY_BEARS), None);
}
