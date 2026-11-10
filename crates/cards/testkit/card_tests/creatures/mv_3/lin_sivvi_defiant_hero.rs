//! `cards/creatures/mv_3/lin_sivvi_defiant_hero.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn lin_sivvi_searches_for_rebel_and_puts_rebel_from_graveyard_on_bottom_of_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, index::NIGHTWIND_GLIDER)
        .battlefield(
            0,
            &[
                card_index("6c3847b6-9bfb-49b5-8f1a-bdb40e6900f1"),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p0, 1);
    let glider_gy = in_graveyard(&engine, p0, index::NIGHTWIND_GLIDER).expect("Glider in gy");
    tap_all_mana(&mut engine, p0);
    activate(
        &mut engine,
        p0,
        card_index("6c3847b6-9bfb-49b5-8f1a-bdb40e6900f1"),
        0,
    );
    engine
        .apply(p0, PlayerAction::ChooseNumber(3))
        .expect("announce X=3");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards");
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("choose Nightwind Glider");
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, index::NIGHTWIND_GLIDER).is_some());

    let lib_before = engine.state().zones.list(ZoneLocation::Library(p0)).len();
    activate(
        &mut engine,
        p0,
        card_index("6c3847b6-9bfb-49b5-8f1a-bdb40e6900f1"),
        1,
    );
    aim_at(&mut engine, p0, glider_gy);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(in_graveyard(&engine, p0, index::NIGHTWIND_GLIDER), None);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        lib_before + 1
    );
}
